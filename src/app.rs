//! État de l'application et logique des écrans. Les opérations lentes (dérivation de clé, base de
//! données, réseau) tournent sur un thread dédié : l'interface ne se fige jamais.

use iced::futures::FutureExt;
use iced::widget::text_editor;
use iced::Task;
use ntm_core::crypto::{self, Cipher, KdfPreset};
use ntm_core::notes::{self, Note, Unlocked};
use ntm_core::plugin::host::{self, Effect, HostInput, NoteView, ThemeInfo, Zone};
use ntm_core::plugin::panel::{PanelValue, Widget};
use ntm_core::plugin::{self, Permission, Plugin};
use ntm_core::settings::{Language, Screen, SecuritySettings, Settings, SyncMode};
use ntm_core::sync::{self, Bundle, Events, Listener, Local, Op, Outcome, ServerShared, SyncEvent};
use ntm_core::theme::{self, Palette, ThemeDef};
use ntm_core::{store, vault};
use rhai::Dynamic;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyfileTarget {
    Unlock,
    Security,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecField {
    Password,
    Confirm,
    Current,
    Backup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeFilter {
    All,
    Dark,
    Light,
}

impl ThemeFilter {
    pub const ALL: [ThemeFilter; 3] = [Self::All, Self::Dark, Self::Light];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginCall {
    pub plugin_id: String,
    pub callback: String,
    pub note_uid: Option<String>,
}

/// Mot de passe actuel et clé en mémoire, pour confirmer une opération sensible sur le coffre.
struct Credentials {
    security: SecuritySettings,
    password: zeroize::Zeroizing<String>,
    keyfile: Option<Vec<u8>>,
    unlocked: Unlocked,
}

pub type Keyfile = Option<(String, Vec<u8>)>;

#[derive(Debug)]
pub enum VaultOp {
    Enabled {
        security: SecuritySettings,
        unlocked: Unlocked,
        notes: Vec<Note>,
    },
    Changed {
        security: SecuritySettings,
        unlocked: Unlocked,
        notes: Vec<Note>,
    },
    Disabled {
        security: SecuritySettings,
        notes: Vec<Note>,
    },
}

#[derive(Debug)]
pub enum SyncOpResult {
    Synced(Vec<String>),
    ConfigPulled(Box<Bundle>, String),
    ConfigPushed(Vec<String>),
}

#[derive(Debug, Clone)]
pub enum Message {
    // Navigation
    NewNote,
    SeeNotes,
    BackHome,
    OpenSettings,
    OpenScreen(Screen),
    // Réglages généraux
    ToggleTheme(bool),
    SaveSettings,
    LanguageChanged(Language),
    ToggleTimestamps(bool),
    ToggleCompact(bool),
    ToggleConfirmDelete(bool),
    FontSizeChanged(u16),
    FontSizeReleased,
    // Écriture et liste des notes
    TitleChanged(String),
    BodyEdited(text_editor::Action),
    SaveNote,
    SaveFinished(Result<Vec<Note>, String>),
    BackFromWrite,
    SearchChanged(String),
    ToggleExpand(String),
    ToggleMenu(String),
    EditNote(String),
    CopyNote(String),
    AskDelete(String),
    ConfirmDelete(String),
    CancelDelete,
    DeleteFinished(Result<Vec<Note>, String>),
    // Coffre
    UnlockPasswordChanged(String),
    Unlock,
    UnlockFinished(Result<(Unlocked, Vec<Note>), String>),
    LockNow,
    PickKeyfile(KeyfileTarget),
    KeyfilePicked(KeyfileTarget, Result<Keyfile, String>),
    ClearKeyfile(KeyfileTarget),
    SecField(SecField, String),
    SecCipher(Cipher),
    SecKdf(KdfPreset),
    AutoLock(u32),
    GeneratePassword,
    EnableVault,
    DisableVault,
    ChangeVault,
    VaultFinished(Result<Arc<Mutex<Option<VaultOp>>>, String>),
    ExportBackup,
    ImportBackup,
    BackupFinished(Result<String, String>),
    ImportFinished(Result<(String, Option<Box<Settings>>), String>),
    // Thèmes
    PreviewTheme(String),
    CancelPreview,
    ApplyTheme(String),
    ThemeFilterChanged(ThemeFilter),
    ThemeSearch(String),
    AccentInput(String),
    RadiusChanged(u16),
    BorderChanged(u16),
    SpacingChanged(u16),
    PanelContrastChanged(i8),
    ThemeSliderReleased,
    ToggleEditorMono(bool),
    ToggleUiMono(bool),
    CustomNameChanged(String),
    SaveCustomTheme,
    // Synchronisation
    SyncToggle(bool),
    SyncName(String),
    SyncPrimary(bool),
    SyncListen(bool),
    SyncPort(String),
    SyncTrusted(bool),
    SyncSecret(String),
    SyncGenerateSecret,
    SyncToggleSecret,
    PeerInput(String),
    AddPeer,
    RemovePeer(usize),
    SyncModeChanged(SyncMode),
    SyncInterval(u32),
    SyncEncrypt(bool),
    SyncOnSave(bool),
    SyncNow,
    SyncTick,
    PullConfig(usize),
    PushConfig,
    SyncOpFinished(Result<Arc<Mutex<Option<SyncOpResult>>>, String>),
    Poll,
    // Plugins
    PluginEnable(String),
    PluginDisable(String),
    PluginEnableBuiltin,
    PluginInstall,
    PluginInstallPicked(Option<PathBuf>),
    PluginRemove(String),
    PluginCall(PluginCall),
    // Divers
    // Panneaux de plugin
    OpenPanel(usize),
    ClosePanel,
    PanelChanged(String, PanelValue),
    PanelButton(String, String),
    Tick,
}

pub struct PluginEntry {
    pub plugin: Plugin,
    pub enabled: bool,
    /// Le plugin était approuvé, mais son code a changé depuis.
    pub changed: bool,
}

pub struct PluginButton {
    pub plugin_id: String,
    pub zone: Zone,
    pub label: String,
    pub callback: String,
}

pub struct PanelEntry {
    pub plugin_id: String,
    pub title: String,
    pub callback: String,
}

pub struct PluginAction {
    pub plugin_id: String,
    pub label: String,
    pub callback: String,
}

pub struct SecurityForm {
    pub password: String,
    pub confirm: String,
    pub current: String,
    pub backup: String,
    pub keyfile: Keyfile,
    pub cipher: Cipher,
    pub preset: KdfPreset,
}

pub struct Paths {
    settings: PathBuf,
    database: PathBuf,
    pub themes: PathBuf,
    pub plugins: PathBuf,
}

pub struct App {
    pub settings: Settings,
    pub themes: Vec<ThemeDef>,
    pub preview: Option<String>,
    pub screen: Screen,
    pub title: String,
    pub body: text_editor::Content,
    pub status: String,
    pub notes: Vec<Note>,
    pub search: String,
    pub expanded: HashSet<String>,
    pub menu_open: Option<String>,
    pub pending_delete: Option<String>,
    // Coffre
    pub vault: Option<Unlocked>,
    pub locked: bool,
    pub busy: bool,
    pub unlock_password: String,
    pub unlock_keyfile: Keyfile,
    pub unlock_error: String,
    pub form: SecurityForm,
    // Thèmes
    pub theme_filter: ThemeFilter,
    pub theme_search: String,
    pub accent_input: String,
    pub custom_name: String,
    // Synchronisation
    pub peer_input: String,
    pub port_input: String,
    pub show_secret: bool,
    pub sync_busy: bool,
    pub sync_log: Vec<String>,
    // Plugins
    pub plugins: Vec<PluginEntry>,
    pub plugin_buttons: Vec<PluginButton>,
    pub note_actions: Vec<PluginAction>,
    pub panels: Vec<PanelEntry>,
    pub active_panel: Option<usize>,
    pub panel_widgets: Vec<Widget>,
    pub panel_state: BTreeMap<String, PanelValue>,
    /// Palette en aperçu direct demandée par le panneau ouvert (Theme Studio…) ; jamais enregistrée.
    pub live_theme: Option<ThemeDef>,
    pub paths: Paths,
    last_activity: Instant,
    editing_uid: Option<String>,
    /// Nombre d'écritures de notes en cours sur un thread : un changement de coffre attend qu'il soit nul.
    io_pending: usize,
    lock_after_save: bool,
    unlock_failures: u32,
    shared: Arc<Mutex<ServerShared>>,
    events: Events,
    listener: Option<Listener>,
}

/// Exécute `work` sur un thread dédié sans bloquer l'exécuteur de l'interface.
pub async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let (sender, receiver) = iced::futures::channel::oneshot::channel();
    std::thread::spawn(move || {
        let _ = sender.send(work());
    });
    receiver
        .await
        .unwrap_or_else(|_| Err("worker stopped unexpectedly".to_string()))
}

fn wrap<T: Send + 'static>(result: Result<T, String>) -> Result<Arc<Mutex<Option<T>>>, String> {
    result.map(|value| Arc::new(Mutex::new(Some(value))))
}

fn take<T>(slot: &Arc<Mutex<Option<T>>>) -> Option<T> {
    slot.lock().ok().and_then(|mut guard| guard.take())
}

pub struct SaveRequest {
    path: PathBuf,
    title: String,
    body: String,
    editing: Option<String>,
    now: String,
    vault: Option<Unlocked>,
}

impl App {
    pub fn new() -> Self {
        let dirs = directories::ProjectDirs::from("com", "NoteToMyself", "NoteToMyself");
        let data_dir = dirs
            .as_ref()
            .map(|d| d.data_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        let config_dir = dirs
            .as_ref()
            .map(|d| d.config_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        let _ = std::fs::create_dir_all(&data_dir);
        let _ = std::fs::create_dir_all(&config_dir);

        let paths = Paths {
            settings: config_dir.join("settings.json"),
            database: data_dir.join("notes.sqlite3"),
            themes: config_dir.join("themes"),
            plugins: config_dir.join("plugins"),
        };
        let mut settings = std::fs::read_to_string(&paths.settings)
            .ok()
            .and_then(|json| serde_json::from_str::<Settings>(&json).ok())
            .unwrap_or_default();
        settings.sanitize();
        // Après une coupure pendant un changement de coffre, la base porte la configuration la plus récente.
        if let Ok(connection) = store::open(&paths.database) {
            if store::ensure_schema(&connection).is_ok() {
                vault::reconcile(&connection, &mut settings.security);
            }
        }

        let mut themes = theme::builtin_themes();
        themes.extend(theme::load_user_themes(&paths.themes));

        let local = Local {
            settings: settings.clone(),
            db_path: paths.database.clone(),
            themes_dir: paths.themes.clone(),
            plugins_dir: paths.plugins.clone(),
        };
        let mut app = Self {
            locked: settings.security.enabled,
            accent_input: settings.theme.accent.clone().unwrap_or_default(),
            port_input: settings.sync.port.to_string(),
            settings,
            themes,
            preview: None,
            screen: Screen::Home,
            title: String::new(),
            body: text_editor::Content::new(),
            status: String::new(),
            notes: Vec::new(),
            search: String::new(),
            expanded: HashSet::new(),
            menu_open: None,
            pending_delete: None,
            vault: None,
            busy: false,
            unlock_password: String::new(),
            unlock_keyfile: None,
            unlock_error: String::new(),
            form: SecurityForm {
                password: String::new(),
                confirm: String::new(),
                current: String::new(),
                backup: String::new(),
                keyfile: None,
                cipher: Cipher::default(),
                preset: KdfPreset::Argon2Standard,
            },
            theme_filter: ThemeFilter::All,
            theme_search: String::new(),
            custom_name: String::new(),
            peer_input: String::new(),
            show_secret: false,
            sync_busy: false,
            sync_log: Vec::new(),
            plugins: Vec::new(),
            plugin_buttons: Vec::new(),
            note_actions: Vec::new(),
            panels: Vec::new(),
            active_panel: None,
            panel_widgets: Vec::new(),
            panel_state: BTreeMap::new(),
            live_theme: None,
            paths,
            last_activity: Instant::now(),
            editing_uid: None,
            io_pending: 0,
            lock_after_save: false,
            unlock_failures: 0,
            shared: Arc::new(Mutex::new(ServerShared::new(local))),
            events: Arc::default(),
            listener: None,
        };
        app.persist_settings();
        app.initialize_database();
        if !app.locked {
            app.load_notes();
        }
        app.reload_plugins();
        app.ensure_listener();
        app
    }

    pub fn t(&self, key: &str) -> String {
        ntm_core::i18n::tr(self.settings.language, key).to_string()
    }

    // --- Apparence ----------------------------------------------------------------------------

    /// Palette affichée : l'aperçu en cours s'il y en a un, sinon le thème choisi avec ses surcharges.
    pub fn palette(&self) -> Palette {
        if let Some(live) = &self.live_theme {
            // Aperçu en direct d'un panneau : le thème tel qu'il serait, sans l'accent personnalisé.
            let mut settings = self.settings.theme.clone();
            settings.accent = None;
            return theme::resolve(live, &settings);
        }
        if let Some(base) = self
            .preview
            .as_deref()
            .and_then(|id| theme::find(&self.themes, id))
        {
            // L'aperçu montre le thème tel qu'il sera une fois appliqué : sans l'accent personnalisé.
            let mut settings = self.settings.theme.clone();
            settings.accent = None;
            return theme::resolve(base, &settings);
        }
        theme::active_palette(&self.themes, &self.settings.theme, self.settings.dark_mode)
    }

    pub fn current_theme_name(&self) -> String {
        let fallback = if self.settings.dark_mode {
            "default-dark"
        } else {
            "default-light"
        };
        theme::find(&self.themes, &self.settings.theme.id)
            .or_else(|| theme::find(&self.themes, fallback))
            .map(|t| t.name.clone())
            .unwrap_or_default()
    }

    fn reload_themes(&mut self) {
        self.themes = theme::builtin_themes();
        self.themes
            .extend(theme::load_user_themes(&self.paths.themes));
    }

    // --- Persistance ----------------------------------------------------------------------------

    pub fn local(&self) -> Local {
        Local {
            settings: self.settings.clone(),
            db_path: self.paths.database.clone(),
            themes_dir: self.paths.themes.clone(),
            plugins_dir: self.paths.plugins.clone(),
        }
    }

    fn persist_settings(&mut self) {
        if let Ok(json) = serde_json::to_string_pretty(&self.settings) {
            // Le fichier peut contenir le secret de synchronisation : lisible par son seul propriétaire,
            // écrit de façon atomique.
            if ntm_core::settings::write_private(&self.paths.settings, json.as_bytes()).is_err() {
                self.status = self.t("save_error");
            }
        }
        if let Ok(mut shared) = self.shared.lock() {
            shared.local = self.local();
        }
    }

    fn initialize_database(&self) {
        if let Ok(connection) = store::open(&self.paths.database) {
            let _ = store::ensure_schema(&connection);
            if !self.settings.sync.enabled {
                // Sans synchronisation, les pierres tombales de plus de 30 jours ne servent à rien.
                let cutoff = (chrono::Local::now() - chrono::Duration::days(30)).to_rfc3339();
                let _ = store::purge_tombstones(&connection, &cutoff);
            }
        }
    }

    pub fn load_notes(&mut self) {
        match notes::reload(&self.paths.database, self.vault.as_ref()) {
            Ok(list) => self.notes = list,
            Err(_) => self.status = self.t("read_error"),
        }
    }

    pub fn update_settings(&mut self) {
        self.set_font_size(self.settings.font_size);
        self.persist_settings();
    }

    pub fn set_font_size(&mut self, size: u16) {
        self.settings.font_size = size.clamp(12, 24);
    }

    fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    // --- Notes -----------------------------------------------------------------------------------

    pub fn start_new_note(&mut self) {
        self.title.clear();
        self.body = text_editor::Content::new();
        self.editing_uid = None;
        self.status.clear();
        self.screen = Screen::Write;
    }

    pub fn is_editing(&self) -> bool {
        self.editing_uid.is_some()
    }

    pub fn open_note(&mut self, uid: &str) {
        if let Some(note) = self.notes.iter().find(|n| n.uid == uid && !n.unreadable) {
            self.title = note.title.clone();
            self.body = text_editor::Content::with_text(&note.body);
            self.editing_uid = Some(note.uid.clone());
            self.status.clear();
            self.menu_open = None;
            self.screen = Screen::Write;
        } else {
            self.status = self.t("unreadable_note");
        }
    }

    pub fn note_text(&self, uid: &str) -> Option<String> {
        self.notes
            .iter()
            .find(|n| n.uid == uid && !n.unreadable)
            .map(|n| {
                if n.title.trim().is_empty() {
                    n.body.clone()
                } else {
                    format!("{}\n\n{}", n.title, n.body)
                }
            })
    }

    /// Prépare l'enregistrement : applique les plugins `on_note_save`, puis délègue à un thread.
    fn prepare_save(&mut self) -> Option<SaveRequest> {
        let mut title = self.title.trim().to_owned();
        let mut body = self.body.text();
        if title.is_empty() && body.trim().is_empty() {
            self.status = self.t("empty_note");
            return None;
        }
        let mut statuses = Vec::new();
        for index in 0..self.plugins.len() {
            let entry = &self.plugins[index];
            let Some(granted) = self.granted(entry) else {
                continue;
            };
            let input = self.host_input(&entry.plugin, &granted);
            match host::run_note_save(&entry.plugin, &granted, input, &title, &body) {
                Ok((new_title, new_body, effects)) => {
                    title = new_title.trim().to_owned();
                    body = new_body;
                    for effect in effects {
                        if let Effect::Status(text) = effect {
                            statuses.push(text);
                        }
                    }
                }
                Err(error) => statuses.push(format!(
                    "{}: {} ({error})",
                    self.t("plugin_error"),
                    entry.plugin.manifest.name
                )),
            }
        }
        self.status = statuses.pop().unwrap_or_else(|| self.t("saving"));
        Some(SaveRequest {
            path: self.paths.database.clone(),
            title,
            body,
            editing: self.editing_uid.clone(),
            now: chrono::Local::now().to_rfc3339(),
            vault: self.vault.clone(),
        })
    }

    fn save_task(&mut self, request: SaveRequest) -> Task<Message> {
        self.io_pending += 1;
        Task::perform(
            blocking(move || {
                notes::save(
                    &request.path,
                    request.editing.as_deref(),
                    &request.title,
                    &request.body,
                    &request.now,
                    request.vault.as_ref(),
                )
            }),
            Message::SaveFinished,
        )
    }

    fn finish_save(&mut self, result: Result<Vec<Note>, String>) -> Task<Message> {
        self.io_pending = self.io_pending.saturating_sub(1);
        let lock_after = std::mem::take(&mut self.lock_after_save);
        match result {
            Ok(list) => {
                self.notes = list;
                self.title.clear();
                self.body = text_editor::Content::new();
                self.editing_uid = None;
                self.status = self.t("saved");
                self.screen = Screen::See;
                if lock_after {
                    // Le brouillon est enregistré (chiffré) : on peut verrouiller sans rien perdre.
                    self.lock();
                }
                self.sync_after_change()
            }
            Err(error) => {
                // L'enregistrement a échoué : on ne verrouille pas, pour ne pas perdre le brouillon.
                self.touch();
                self.status = format!("{} ({error})", self.t("save_error"));
                Task::none()
            }
        }
    }

    /// Verrouillage après inactivité. Un brouillon en cours est d'abord enregistré (chiffré).
    fn auto_lock(&mut self) -> Task<Message> {
        if self.busy || self.io_pending > 0 {
            return Task::none();
        }
        let has_draft = !self.title.trim().is_empty() || !self.body.text().trim().is_empty();
        if has_draft {
            if let Some(request) = self.prepare_save() {
                self.lock_after_save = true;
                return self.save_task(request);
            }
        }
        self.lock();
        Task::none()
    }

    fn delete_task(&mut self, uid: String) -> Task<Message> {
        self.pending_delete = None;
        self.io_pending += 1;
        let (path, vault) = (self.paths.database.clone(), self.vault.clone());
        let now = chrono::Local::now().to_rfc3339();
        Task::perform(
            blocking(move || notes::delete(&path, &uid, &now, vault.as_ref())),
            Message::DeleteFinished,
        )
    }

    fn finish_delete(&mut self, result: Result<Vec<Note>, String>) -> Task<Message> {
        self.io_pending = self.io_pending.saturating_sub(1);
        self.pending_delete = None;
        match result {
            Ok(list) => {
                self.expanded
                    .retain(|uid| list.iter().any(|n| &n.uid == uid));
                self.notes = list;
                self.status = self.t("deleted");
                self.sync_after_change()
            }
            Err(error) => {
                self.status = format!("{} ({error})", self.t("delete_error"));
                Task::none()
            }
        }
    }

    // --- Coffre ------------------------------------------------------------------------------------

    pub fn lock(&mut self) {
        if !self.settings.security.enabled {
            return;
        }
        self.vault = None;
        self.locked = true;
        self.notes.clear();
        self.search.clear();
        self.expanded.clear();
        self.menu_open = None;
        self.pending_delete = None;
        self.title.clear();
        self.body = text_editor::Content::new();
        self.editing_uid = None;
        self.plugin_buttons.clear();
        self.note_actions.clear();
        self.panels.clear();
        self.active_panel = None;
        self.panel_widgets.clear();
        self.panel_state.clear();
        self.live_theme = None;
        self.unlock_keyfile = None;
        self.form.current.clear();
        self.form.password.clear();
        self.form.confirm.clear();
        self.form.backup.clear();
        self.form.keyfile = None;
        self.screen = Screen::Home;
        self.status.clear();
    }

    fn start_unlock(&mut self) -> Task<Message> {
        if self.busy {
            return Task::none();
        }
        self.busy = true;
        self.unlock_error.clear();
        let security = self.settings.security.clone();
        let password = zeroize::Zeroizing::new(std::mem::take(&mut self.unlock_password));
        let keyfile = self.unlock_keyfile.as_ref().map(|(_, bytes)| bytes.clone());
        let path = self.paths.database.clone();
        // Après trois échecs, chaque nouvel essai attend de plus en plus longtemps (30 s au maximum).
        let delay = match self.unlock_failures {
            0..=2 => Duration::ZERO,
            n => Duration::from_secs((1u64 << (n - 3).min(5)).min(30)),
        };
        Task::perform(
            blocking(move || {
                if !delay.is_zero() {
                    std::thread::sleep(delay);
                }
                let key = vault::unlock(&security, &password, keyfile.as_deref())
                    .map_err(|e| e.to_string())?;
                let connection = store::open(&path).map_err(|e| e.to_string())?;
                store::ensure_schema(&connection).map_err(|e| e.to_string())?;
                // Chiffre au passage les notes encore en clair (reçues d'un PC sans coffre, import…).
                vault::encrypt_database(&connection, &key, security.cipher, None)
                    .map_err(|e| e.to_string())?;
                let unlocked = Unlocked {
                    key,
                    cipher: security.cipher,
                };
                let list = notes::load(&connection, Some(&unlocked)).map_err(|e| e.to_string())?;
                Ok((unlocked, list))
            }),
            Message::UnlockFinished,
        )
    }

    fn finish_unlock(&mut self, result: Result<(Unlocked, Vec<Note>), String>) {
        self.busy = false;
        match result {
            Ok((unlocked, list)) => {
                self.unlock_failures = 0;
                self.vault = Some(unlocked);
                self.notes = list;
                self.locked = false;
                self.unlock_error.clear();
                self.touch();
                self.reload_plugin_ui();
            }
            Err(error) => {
                if error.contains("wrong password") {
                    self.unlock_failures += 1;
                }
                self.unlock_error = if error.contains("wrong password") {
                    self.t("wrong_password")
                } else {
                    error
                };
            }
        }
    }

    fn validate_new_password(&self) -> Result<(), String> {
        if self.form.password != self.form.confirm {
            return Err(self.t("passwords_differ"));
        }
        if self.form.password.chars().count() < 10
            || crypto::password_strength(&self.form.password) < 1
        {
            return Err(self.t("password_weak"));
        }
        Ok(())
    }

    fn current_keyfile_bytes(&self) -> Option<Vec<u8>> {
        self.unlock_keyfile.as_ref().map(|(_, b)| b.clone())
    }

    /// Un changement de coffre ne démarre que si rien d'autre n'écrit dans la base : une note enregistrée
    /// avec l'ancienne clé pendant le rechiffrement deviendrait illisible. Pendant l'opération, la
    /// synchronisation entrante est aussi refusée.
    fn begin_vault_operation(&mut self) -> bool {
        if self.busy || self.sync_busy || self.io_pending > 0 {
            self.status = self.t("busy");
            return false;
        }
        self.busy = true;
        self.status = self.t("busy");
        self.set_maintenance(true);
        true
    }

    fn set_maintenance(&self, on: bool) {
        if let Ok(mut shared) = self.shared.lock() {
            shared.maintenance = on;
        }
    }

    fn start_enable_vault(&mut self) -> Task<Message> {
        if self.busy || self.settings.security.enabled {
            return Task::none();
        }
        if let Err(message) = self.validate_new_password() {
            self.status = message;
            return Task::none();
        }
        if !self.begin_vault_operation() {
            return Task::none();
        }
        let (password, keyfile) = (
            zeroize::Zeroizing::new(self.form.password.clone()),
            self.form.keyfile.as_ref().map(|(_, b)| b.clone()),
        );
        let (cipher, kdf) = (self.form.cipher, self.form.preset.kdf());
        let (previous, path) = (self.settings.security.clone(), self.paths.database.clone());
        Task::perform(
            blocking(move || {
                let (security, key) =
                    vault::create(&password, keyfile.as_deref(), cipher, kdf, &previous)
                        .map_err(|e| e.to_string())?;
                let connection = store::open(&path).map_err(|e| e.to_string())?;
                store::ensure_schema(&connection).map_err(|e| e.to_string())?;
                // La configuration du coffre est écrite dans la même transaction que les notes chiffrées.
                vault::encrypt_database(&connection, &key, cipher, Some(&security))
                    .map_err(|e| e.to_string())?;
                let unlocked = Unlocked { key, cipher };
                let list = notes::load(&connection, Some(&unlocked)).map_err(|e| e.to_string())?;
                Ok(VaultOp::Enabled {
                    security,
                    unlocked,
                    notes: list,
                })
            })
            .map(wrap),
            Message::VaultFinished,
        )
    }

    fn current_credentials(&mut self) -> Option<Credentials> {
        let unlocked = self.vault.clone()?;
        let password = zeroize::Zeroizing::new(std::mem::take(&mut self.form.current));
        Some(Credentials {
            security: self.settings.security.clone(),
            password,
            keyfile: self.current_keyfile_bytes(),
            unlocked,
        })
    }

    fn start_disable_vault(&mut self) -> Task<Message> {
        if self.busy || !self.settings.security.enabled || self.vault.is_none() {
            return Task::none();
        }
        if !self.begin_vault_operation() {
            return Task::none();
        }
        let Some(Credentials {
            security,
            password,
            keyfile,
            unlocked,
        }) = self.current_credentials()
        else {
            self.busy = false;
            self.set_maintenance(false);
            return Task::none();
        };
        let disabled = SecuritySettings {
            auto_lock_minutes: security.auto_lock_minutes,
            rev: security.rev + 1,
            ..Default::default()
        };
        let path = self.paths.database.clone();
        Task::perform(
            blocking(move || {
                vault::unlock(&security, &password, keyfile.as_deref())
                    .map_err(|e| e.to_string())?;
                let connection = store::open(&path).map_err(|e| e.to_string())?;
                vault::decrypt_database(
                    &connection,
                    &unlocked.key,
                    unlocked.cipher,
                    Some(&disabled),
                )
                .map_err(|e| e.to_string())?;
                let list = notes::load(&connection, None).map_err(|e| e.to_string())?;
                Ok(VaultOp::Disabled {
                    security: disabled,
                    notes: list,
                })
            })
            .map(wrap),
            Message::VaultFinished,
        )
    }

    fn start_change_vault(&mut self) -> Task<Message> {
        if self.busy || !self.settings.security.enabled || self.vault.is_none() {
            return Task::none();
        }
        if let Err(message) = self.validate_new_password() {
            self.status = message;
            return Task::none();
        }
        if !self.begin_vault_operation() {
            return Task::none();
        }
        let (new_password, new_keyfile) = (
            zeroize::Zeroizing::new(self.form.password.clone()),
            self.form.keyfile.as_ref().map(|(_, b)| b.clone()),
        );
        let (cipher, kdf) = (self.form.cipher, self.form.preset.kdf());
        let Some(Credentials {
            security,
            password,
            keyfile,
            unlocked,
        }) = self.current_credentials()
        else {
            self.busy = false;
            self.set_maintenance(false);
            return Task::none();
        };
        let path = self.paths.database.clone();
        Task::perform(
            blocking(move || {
                vault::unlock(&security, &password, keyfile.as_deref())
                    .map_err(|e| e.to_string())?;
                let (new_security, new_key) = vault::create(
                    &new_password,
                    new_keyfile.as_deref(),
                    cipher,
                    kdf,
                    &security,
                )
                .map_err(|e| e.to_string())?;
                let connection = store::open(&path).map_err(|e| e.to_string())?;
                vault::rekey_database(
                    &connection,
                    &unlocked.key,
                    unlocked.cipher,
                    &new_key,
                    cipher,
                    Some(&new_security),
                )
                .map_err(|e| e.to_string())?;
                let new_unlocked = Unlocked {
                    key: new_key,
                    cipher,
                };
                let list =
                    notes::load(&connection, Some(&new_unlocked)).map_err(|e| e.to_string())?;
                Ok(VaultOp::Changed {
                    security: new_security,
                    unlocked: new_unlocked,
                    notes: list,
                })
            })
            .map(wrap),
            Message::VaultFinished,
        )
    }

    fn finish_vault(&mut self, result: Result<Arc<Mutex<Option<VaultOp>>>, String>) {
        self.busy = false;
        self.set_maintenance(false);
        let op = match result {
            Ok(slot) => take(&slot),
            Err(error) => {
                self.status = if error.contains("wrong password") {
                    self.t("wrong_password")
                } else {
                    error
                };
                return;
            }
        };
        let keyfile = self.form.keyfile.clone();
        match op {
            Some(VaultOp::Enabled {
                security,
                unlocked,
                notes,
            })
            | Some(VaultOp::Changed {
                security,
                unlocked,
                notes,
            }) => {
                let changed = self.settings.security.enabled;
                self.settings.security = security;
                self.vault = Some(unlocked);
                self.notes = notes;
                self.unlock_keyfile = keyfile;
                self.status = self.t(if changed {
                    "vault_changed"
                } else {
                    "vault_enabled_done"
                });
            }
            Some(VaultOp::Disabled { security, notes }) => {
                self.settings.security = security;
                self.vault = None;
                self.unlock_keyfile = None;
                self.notes = notes;
                self.status = self.t("vault_removed");
            }
            None => {}
        }
        self.form.password.clear();
        self.form.confirm.clear();
        self.form.current.clear();
        self.form.keyfile = None;
        self.persist_settings();
        self.ensure_listener();
    }

    fn pick_keyfile(target: KeyfileTarget) -> Task<Message> {
        Task::perform(
            async move {
                let Some(file) = rfd::AsyncFileDialog::new().pick_file().await else {
                    return Ok(None);
                };
                let bytes = file.read().await;
                if bytes.len() > 1024 * 1024 {
                    return Err("the key file is larger than 1 MiB".to_string());
                }
                Ok(Some((file.file_name(), bytes)))
            },
            move |result| Message::KeyfilePicked(target, result),
        )
    }

    fn export_backup(&mut self) -> Task<Message> {
        if self.vault.is_none() && self.settings.security.enabled {
            return Task::none();
        }
        let password = self.form.backup.clone();
        if password.chars().count() < 10 {
            self.status = self.t("password_weak");
            return Task::none();
        }
        let list: Vec<vault::BackupNote> = self
            .notes
            .iter()
            .filter(|n| !n.unreadable)
            .map(|n| vault::BackupNote {
                uid: n.uid.clone(),
                title: n.title.clone(),
                body: n.body.clone(),
                created_at: n.created_at.clone(),
                updated_at: n.updated_at.clone(),
            })
            .collect();
        let settings = self.settings.clone();
        let kdf = self.form.preset.kdf();
        Task::perform(
            async move {
                let bytes = blocking(move || {
                    vault::export_backup(list, &settings, &password, Cipher::Aes256Gcm, kdf)
                        .map_err(|e| e.to_string())
                })
                .await?;
                let Some(handle) = rfd::AsyncFileDialog::new()
                    .set_file_name("notes.ntmbackup")
                    .save_file()
                    .await
                else {
                    return Err(String::new());
                };
                handle.write(&bytes).await.map_err(|e| e.to_string())?;
                Ok("backup_saved".to_string())
            },
            Message::BackupFinished,
        )
    }

    fn import_backup(&mut self) -> Task<Message> {
        if self.settings.security.enabled && self.vault.is_none() {
            return Task::none();
        }
        let password = zeroize::Zeroizing::new(self.form.backup.clone());
        let (path, vault_handle) = (self.paths.database.clone(), self.vault.clone());
        Task::perform(
            async move {
                let Some(file) = rfd::AsyncFileDialog::new().pick_file().await else {
                    return Err(String::new());
                };
                let bytes = file.read().await;
                blocking(move || {
                    let (backup_notes, settings) =
                        vault::import_backup(&bytes, &password).map_err(|e| e.to_string())?;
                    let connection = store::open(&path).map_err(|e| e.to_string())?;
                    store::ensure_schema(&connection).map_err(|e| e.to_string())?;
                    // Tout ou rien : un import interrompu ne laisse pas la moitié des notes.
                    let transaction = connection
                        .unchecked_transaction()
                        .map_err(|e| e.to_string())?;
                    for note in backup_notes {
                        if !store::valid_uid(&note.uid) {
                            continue;
                        }
                        // Les notes déjà présentes ne sont jamais écrasées ; une note supprimée est rétablie.
                        let existing = store::record_by_uid(&transaction, &note.uid)
                            .map_err(|e| e.to_string())?;
                        if existing.as_ref().is_some_and(|e| !e.deleted) {
                            continue;
                        }
                        let mut record = store::Record {
                            id: 0,
                            uid: note.uid,
                            title: note.title,
                            body: note.body,
                            created_at: note.created_at,
                            updated_at: chrono::Local::now().to_rfc3339(),
                            deleted: false,
                            rev: 1,
                        };
                        if let Some(v) = vault_handle.as_ref() {
                            vault::encrypt_record_fields(&v.key, v.cipher, &mut record)
                                .map_err(|e| e.to_string())?;
                        }
                        store::upsert_remote(&transaction, &record).map_err(|e| e.to_string())?;
                    }
                    transaction.commit().map_err(|e| e.to_string())?;
                    Ok(("backup_imported".to_string(), settings.map(Box::new)))
                })
                .await
            },
            Message::ImportFinished,
        )
    }

    /// Reprend l'apparence et les préférences d'affichage d'une sauvegarde. Le coffre, la synchronisation
    /// et les plugins ne sont jamais modifiés par un import.
    fn apply_backup_appearance(&mut self, backup: Settings) {
        let s = &mut self.settings;
        s.language = backup.language;
        s.dark_mode = backup.dark_mode;
        s.theme = backup.theme;
        s.font_size = backup.font_size;
        s.compact_mode = backup.compact_mode;
        s.show_timestamps = backup.show_timestamps;
        s.confirm_delete = backup.confirm_delete;
        self.accent_input = self.settings.theme.accent.clone().unwrap_or_default();
        self.persist_settings();
    }

    // --- Thèmes -------------------------------------------------------------------------------------

    fn apply_theme(&mut self, id: String) {
        self.live_theme = None;
        self.settings.theme.id = id.clone();
        self.settings.theme.accent = None;
        self.accent_input.clear();
        if let Some(theme) = theme::find(&self.themes, &id) {
            self.settings.dark_mode = theme.dark;
        }
        self.preview = None;
        self.persist_settings();
        self.status = self.t("theme_applied");
    }

    fn save_custom_theme(&mut self) {
        let name = self.custom_name.trim().to_string();
        if name.is_empty() {
            return;
        }
        let palette = self.palette();
        let draft = ThemeDef {
            id: String::new(),
            name,
            dark: palette.dark,
            bg: palette.bg,
            fg: palette.fg,
            accent: palette.accent,
        };
        if self.store_theme(draft) {
            self.custom_name.clear();
        }
    }

    /// Enregistre un thème personnalisé (nom et couleurs validés) et l'applique.
    fn store_theme(&mut self, draft: ThemeDef) -> bool {
        let saved = theme::user_theme_from_json(&theme::user_theme_to_json(&draft))
            .map(|t| theme::save_user_theme(&self.paths.themes, &t).map(|_| t));
        match saved {
            Some(Ok(theme)) => {
                self.reload_themes();
                self.settings.theme.id = theme.id;
                self.settings.theme.accent = None;
                self.accent_input.clear();
                self.live_theme = None;
                self.persist_settings();
                self.status = self.t("theme_saved");
                true
            }
            _ => {
                self.status = self.t("save_error");
                false
            }
        }
    }

    // --- Synchronisation --------------------------------------------------------------------------

    pub fn sync_ready(&self) -> bool {
        let s = &self.settings.sync;
        s.enabled && s.secret.chars().count() >= 8 && !s.peers.is_empty()
    }

    pub fn sync_interval(&self) -> Option<Duration> {
        self.sync_ready()
            .then(|| Duration::from_secs(self.settings.sync.interval_secs as u64))
    }

    fn ensure_listener(&mut self) {
        let sync = &self.settings.sync;
        let wanted = sync.enabled && sync.listen && sync.secret.chars().count() >= 8;
        let port = sync.port;
        match (&self.listener, wanted) {
            (Some(l), true) if l.port == port => {}
            (_, false) => self.listener = None,
            (_, true) => {
                self.listener = None;
                match sync::start_listener(port, self.shared.clone(), self.events.clone()) {
                    Ok(listener) => self.listener = Some(listener),
                    Err(error) => self.log(format!("cannot listen on port {port}: {error}")),
                }
            }
        }
    }

    fn log(&mut self, line: String) {
        self.sync_log.push(format!(
            "{}  {line}",
            chrono::Local::now().format("%H:%M:%S")
        ));
        let excess = self.sync_log.len().saturating_sub(40);
        self.sync_log.drain(..excess);
    }

    fn sync_after_change(&mut self) -> Task<Message> {
        if self.settings.sync.sync_on_save {
            self.start_sync()
        } else {
            Task::none()
        }
    }

    fn start_sync(&mut self) -> Task<Message> {
        if self.sync_busy || self.busy || !self.sync_ready() {
            return Task::none();
        }
        self.sync_busy = true;
        let (local, peers) = (self.local(), self.settings.sync.peers.clone());
        Task::perform(
            blocking(move || {
                let mut lines = Vec::new();
                for peer in peers {
                    match sync::run_client(&local, &peer, Op::Sync) {
                        Ok(Outcome::Synced(report)) => lines.push(report.to_string()),
                        Ok(_) => {}
                        Err(error) => lines.push(format!("{peer}: {error}")),
                    }
                }
                Ok(SyncOpResult::Synced(lines))
            })
            .map(wrap),
            Message::SyncOpFinished,
        )
    }

    fn start_pull_config(&mut self, index: usize) -> Task<Message> {
        let Some(peer) = self.settings.sync.peers.get(index).cloned() else {
            return Task::none();
        };
        if self.sync_busy || self.settings.sync.secret.chars().count() < 8 {
            self.status = self.t("sync_needs_secret");
            return Task::none();
        }
        self.sync_busy = true;
        let local = self.local();
        Task::perform(
            blocking(
                move || match sync::run_client(&local, &peer, Op::ConfigPull) {
                    Ok(Outcome::ConfigPulled(bundle, name)) => {
                        Ok(SyncOpResult::ConfigPulled(bundle, name))
                    }
                    Ok(_) => Err("unexpected reply".to_string()),
                    Err(error) => Err(format!("{peer}: {error}")),
                },
            )
            .map(wrap),
            Message::SyncOpFinished,
        )
    }

    fn start_push_config(&mut self) -> Task<Message> {
        if self.sync_busy || !self.sync_ready() {
            self.status = self.t("sync_needs_secret");
            return Task::none();
        }
        self.sync_busy = true;
        let (local, peers) = (self.local(), self.settings.sync.peers.clone());
        Task::perform(
            blocking(move || {
                let lines = peers
                    .iter()
                    .map(
                        |peer| match sync::run_client(&local, peer, Op::ConfigPush) {
                            Ok(_) => format!("{peer}: ok"),
                            Err(error) => format!("{peer}: {error}"),
                        },
                    )
                    .collect();
                Ok(SyncOpResult::ConfigPushed(lines))
            })
            .map(wrap),
            Message::SyncOpFinished,
        )
    }

    fn finish_sync_op(&mut self, result: Result<Arc<Mutex<Option<SyncOpResult>>>, String>) {
        self.sync_busy = false;
        match result.map(|slot| take(&slot)) {
            Ok(Some(SyncOpResult::Synced(lines))) => {
                for line in lines {
                    self.log(line);
                }
                if !self.locked {
                    self.load_notes();
                }
            }
            Ok(Some(SyncOpResult::ConfigPulled(bundle, from))) => {
                self.apply_bundle(*bundle);
                self.log(format!("{from}: {}", self.t("config_applied")));
                self.status = self.t("config_applied");
            }
            Ok(Some(SyncOpResult::ConfigPushed(lines))) => {
                for line in lines {
                    self.log(line);
                }
                self.status = self.t("config_sent");
            }
            Ok(None) => {}
            Err(error) => self.log(error),
        }
    }

    /// Applique une configuration reçue d'un autre PC (récupérée ou poussée par le PC principal).
    fn apply_bundle(&mut self, bundle: Bundle) {
        let security_before = self.settings.security.clone();
        sync::install_bundle_files(&bundle, &self.paths.themes, &self.paths.plugins);
        if self.settings.apply_transfer(bundle.settings) {
            self.log(self.t("downgrade_refused"));
        }
        self.accent_input = self.settings.theme.accent.clone().unwrap_or_default();
        self.persist_settings();
        self.reload_themes();
        self.reload_plugins();
        self.ensure_listener();
        if self.settings.security != security_before {
            // Un autre coffre : la clé en mémoire ne correspond plus, il faut se déverrouiller à nouveau.
            self.vault = None;
            self.notes.clear();
            self.locked = self.settings.security.enabled;
            if !self.locked {
                self.load_notes();
            }
        }
    }

    fn poll_events(&mut self) {
        let events: Vec<SyncEvent> = self
            .events
            .lock()
            .map(|mut q| std::mem::take(&mut *q))
            .unwrap_or_default();
        for event in events {
            match event {
                SyncEvent::NotesChanged => {
                    if !self.locked {
                        self.load_notes();
                    }
                }
                SyncEvent::ConfigReceived(bundle) => {
                    self.apply_bundle(*bundle);
                    self.log(self.t("config_applied"));
                }
                SyncEvent::Log(line) => self.log(line),
            }
        }
    }

    // --- Plugins --------------------------------------------------------------------------------------

    pub fn reload_plugins(&mut self) {
        let mut loaded = plugin::builtin::load_all();
        let builtin_ids: HashSet<String> = loaded.iter().map(|p| p.manifest.id.clone()).collect();
        if let Ok(dir) = std::fs::read_dir(&self.paths.plugins) {
            let mut folders: Vec<PathBuf> = dir
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect();
            folders.sort();
            let mut seen = builtin_ids.clone();
            for folder in folders {
                // Un identifiant déjà pris (intégré ou autre dossier) est ignoré : les approbations
                // sont rattachées à l'identifiant et ne doivent pas être ambiguës.
                if let Ok(p) = Plugin::load_dir(&folder) {
                    if seen.insert(p.manifest.id.clone()) {
                        loaded.push(p);
                    }
                }
            }
        }
        self.plugins = loaded
            .into_iter()
            .map(|plugin| {
                let approved = self.settings.plugins.approved.get(&plugin.manifest.id);
                PluginEntry {
                    enabled: approved == Some(&plugin.hash),
                    changed: approved.is_some() && approved != Some(&plugin.hash),
                    plugin,
                }
            })
            .collect();
        self.reload_plugin_ui();
    }

    /// Droits d'un plugin : exactement ce que l'analyse du code a détecté, et seulement s'il est approuvé.
    fn granted(&self, entry: &PluginEntry) -> Option<BTreeSet<Permission>> {
        entry
            .enabled
            .then(|| entry.plugin.analysis.permissions.clone())
    }

    fn host_input(&self, plugin: &Plugin, granted: &BTreeSet<Permission>) -> HostInput {
        let mut input = HostInput {
            theme_id: String::new(),
            ..Default::default()
        };
        if granted.contains(&Permission::NotesRead) {
            // Dans l'ordre d'affichage inversé : `notes_list` les renvoie du plus récent au plus ancien.
            input.notes = self
                .notes
                .iter()
                .rev()
                .filter(|n| !n.unreadable)
                .map(|n| NoteView {
                    uid: n.uid.clone(),
                    title: n.title.clone(),
                    body: n.body.clone(),
                    created_at: n.created_at.clone(),
                    updated_at: n.updated_at.clone(),
                })
                .collect();
        }
        if granted.contains(&Permission::EditorRead) {
            input.editor_title = self.title.clone();
            input.editor_text = self.body.text();
        }
        if granted.contains(&Permission::SettingsRead) {
            input.theme_id = self.settings.theme.id.clone();
            input.themes = self
                .themes
                .iter()
                .map(|t| ThemeInfo {
                    id: t.id.clone(),
                    name: t.name.clone(),
                    dark: t.dark,
                    bg: t.bg.to_hex(),
                    fg: t.fg.to_hex(),
                    accent: t.accent.to_hex(),
                })
                .collect();
            let s = &self.settings;
            input.settings = BTreeMap::from([
                ("language".to_string(), s.language.to_string()),
                ("font_size".to_string(), s.font_size.to_string()),
                ("dark_mode".to_string(), s.dark_mode.to_string()),
                ("compact_mode".to_string(), s.compact_mode.to_string()),
            ]);
        }
        if granted.contains(&Permission::Storage) {
            if let Ok(connection) = store::open(&self.paths.database) {
                for (owner, key, value) in store::kv_all(&connection).unwrap_or_default() {
                    if owner != plugin.manifest.id {
                        continue;
                    }
                    let plain = match self.vault.as_ref() {
                        Some(v) => {
                            vault::decrypt_value(&v.key, v.cipher, &owner, &key, &value).ok()
                        }
                        None => (!crypto::is_encrypted(&value)).then_some(value),
                    };
                    if let Some(plain) = plain {
                        input.storage.insert(key, plain);
                    }
                }
            }
        }
        input
    }

    /// Relance `on_start` de chaque plugin actif pour reconstruire les boutons et actions.
    fn reload_plugin_ui(&mut self) {
        self.plugin_buttons.clear();
        self.note_actions.clear();
        self.panels.clear();
        self.active_panel = None;
        self.panel_widgets.clear();
        self.panel_state.clear();
        self.live_theme = None;
        if self.locked {
            return;
        }
        let mut deferred = Vec::new();
        for index in 0..self.plugins.len() {
            let entry = &self.plugins[index];
            let Some(granted) = self.granted(entry) else {
                continue;
            };
            let input = self.host_input(&entry.plugin, &granted);
            let id = entry.plugin.manifest.id.clone();
            match host::run_start(&entry.plugin, &granted, input) {
                Ok(effects) => deferred.push((id, effects)),
                Err(error) => {
                    self.status = format!(
                        "{}: {} ({error})",
                        self.t("plugin_error"),
                        entry.plugin.manifest.name
                    )
                }
            }
        }
        for (id, effects) in deferred {
            // Les actions demandées au démarrage ne produisent pas de tâche asynchrone ici.
            let _ = self.apply_effects(&id, effects, true);
        }
    }

    /// Applique les effets demandés par un plugin. Au démarrage (`start`), un plugin ne peut qu'enregistrer
    /// son interface (boutons, actions) et afficher un message : il ne modifie ni les notes ni le
    /// presse-papiers sans action de l'utilisateur. Hors démarrage, il ne peut pas ajouter de boutons.
    fn apply_effects(
        &mut self,
        plugin_id: &str,
        effects: Vec<Effect>,
        start: bool,
    ) -> Task<Message> {
        const MAX_BUTTONS: usize = 8;
        const MAX_ACTIONS: usize = 4;
        let mut tasks = Vec::new();
        let mut notes_changed = false;
        for effect in effects {
            match effect {
                Effect::Status(text) => self.status = text,
                Effect::Button {
                    zone,
                    label,
                    callback,
                } => {
                    let count = self
                        .plugin_buttons
                        .iter()
                        .filter(|b| b.plugin_id == plugin_id)
                        .count();
                    if start && count < MAX_BUTTONS {
                        self.plugin_buttons.push(PluginButton {
                            plugin_id: plugin_id.to_string(),
                            zone,
                            label,
                            callback,
                        });
                    }
                }
                Effect::NoteAction { label, callback } => {
                    let count = self
                        .note_actions
                        .iter()
                        .filter(|a| a.plugin_id == plugin_id)
                        .count();
                    if start && count < MAX_ACTIONS {
                        self.note_actions.push(PluginAction {
                            plugin_id: plugin_id.to_string(),
                            label,
                            callback,
                        });
                    }
                }
                Effect::Panel { title, callback } => {
                    let count = self
                        .panels
                        .iter()
                        .filter(|p| p.plugin_id == plugin_id)
                        .count();
                    if start && count < 2 {
                        self.panels.push(PanelEntry {
                            plugin_id: plugin_id.to_string(),
                            title,
                            callback,
                        });
                    }
                }
                Effect::PanelSet { id, value } => {
                    // L'état d'un panneau est borné : un plugin ne peut pas le faire grossir sans limite.
                    let known = self.panel_state.contains_key(&id);
                    if self.active_panel.is_some()
                        && !start
                        && (known || self.panel_state.len() < 64)
                    {
                        self.panel_state.insert(id, value);
                    }
                }
                Effect::ThemePreview {
                    dark,
                    bg,
                    fg,
                    accent,
                } => {
                    if self.active_panel.is_some() && !start {
                        self.live_theme = theme::theme_from_hex("preview", dark, &bg, &fg, &accent);
                    }
                }
                Effect::ThemePreviewClear => self.live_theme = None,
                Effect::ThemeSave {
                    name,
                    dark,
                    bg,
                    fg,
                    accent,
                } => {
                    if !start {
                        if let Some(draft) = theme::theme_from_hex(&name, dark, &bg, &fg, &accent) {
                            self.store_theme(draft);
                        }
                    }
                }
                Effect::SetTheme(id) => {
                    if theme::find(&self.themes, &id).is_some() {
                        self.apply_theme(id);
                    }
                }
                Effect::StorageSet { key, value } => {
                    self.plugin_storage_write(plugin_id, &key, &value)
                }
                // Les effets suivants n'ont de sens qu'en réponse à une action de l'utilisateur.
                _ if start => {}
                Effect::Clipboard(text) => tasks.push(iced::clipboard::write(text)),
                Effect::AddNote { title, body } => {
                    notes_changed |= self.plugin_note_write(None, &title, &body);
                }
                Effect::UpdateNote { uid, title, body } => {
                    notes_changed |= self.plugin_note_write(Some(&uid), &title, &body);
                }
                Effect::DeleteNote(uid) => {
                    let now = chrono::Local::now().to_rfc3339();
                    if let Ok(list) =
                        notes::delete(&self.paths.database, &uid, &now, self.vault.as_ref())
                    {
                        self.notes = list;
                        notes_changed = true;
                    }
                }
                Effect::EditorSetTitle(text) => self.title = text,
                Effect::EditorSetText(text) => self.body = text_editor::Content::with_text(&text),
                Effect::EditorInsert(text) => {
                    self.body
                        .perform(text_editor::Action::Edit(text_editor::Edit::Paste(
                            Arc::new(text),
                        )))
                }
            }
        }
        if notes_changed {
            tasks.push(self.sync_after_change());
        }
        Task::batch(tasks)
    }

    fn plugin_note_write(&mut self, uid: Option<&str>, title: &str, body: &str) -> bool {
        let now = chrono::Local::now().to_rfc3339();
        match notes::save(
            &self.paths.database,
            uid,
            title,
            body,
            &now,
            self.vault.as_ref(),
        ) {
            Ok(list) => {
                self.notes = list;
                true
            }
            Err(error) => {
                self.status = format!("{}: {error}", self.t("plugin_error"));
                false
            }
        }
    }

    fn plugin_storage_write(&mut self, plugin_id: &str, key: &str, value: &str) {
        const MAX_KEYS: usize = 256;
        let Ok(connection) = store::open(&self.paths.database) else {
            return;
        };
        let known = store::kv_all(&connection).unwrap_or_default();
        let owned = known.iter().filter(|(p, _, _)| p == plugin_id).count();
        let exists = known.iter().any(|(p, k, _)| p == plugin_id && k == key);
        if !exists && owned >= MAX_KEYS {
            self.status = format!("{}: storage full", self.t("plugin_error"));
            return;
        }
        let stored = match self.vault.as_ref() {
            Some(v) => match vault::encrypt_value(&v.key, v.cipher, plugin_id, key, value) {
                Ok(sealed) => sealed,
                Err(_) => return,
            },
            None => value.to_string(),
        };
        let _ = store::kv_set(&connection, plugin_id, key, &stored);
    }

    fn call_plugin(&mut self, call: PluginCall) -> Task<Message> {
        let Some(entry) = self
            .plugins
            .iter()
            .find(|p| p.plugin.manifest.id == call.plugin_id)
        else {
            return Task::none();
        };
        let Some(granted) = self.granted(entry) else {
            return Task::none();
        };
        let input = self.host_input(&entry.plugin, &granted);
        let args = match &call.note_uid {
            Some(uid) => match self.notes.iter().find(|n| &n.uid == uid && !n.unreadable) {
                Some(note) => {
                    let view = NoteView {
                        uid: note.uid.clone(),
                        title: note.title.clone(),
                        body: note.body.clone(),
                        created_at: note.created_at.clone(),
                        updated_at: note.updated_at.clone(),
                    };
                    vec![view.to_map().into()]
                }
                None => return Task::none(),
            },
            None => vec![],
        };
        // Seuls les rappels réellement demandés par le plugin (boutons, actions) peuvent être appelés.
        let known = self
            .plugin_buttons
            .iter()
            .any(|b| b.plugin_id == call.plugin_id && b.callback == call.callback)
            || self
                .note_actions
                .iter()
                .any(|a| a.plugin_id == call.plugin_id && a.callback == call.callback);
        if !known {
            return Task::none();
        }
        self.menu_open = None;
        match host::run(&entry.plugin, &granted, input, &call.callback, args) {
            Ok(outcome) => self.apply_effects(&call.plugin_id, outcome.effects, false),
            Err(error) => {
                self.status = format!("{}: {} ({error})", self.t("plugin_error"), call.plugin_id);
                Task::none()
            }
        }
    }

    // --- Panneaux de plugin ----------------------------------------------------------------------

    fn open_panel(&mut self, index: usize) -> Task<Message> {
        if index >= self.panels.len() {
            return Task::none();
        }
        self.panel_state.clear();
        self.panel_widgets.clear();
        self.live_theme = None;
        self.active_panel = Some(index);
        self.status.clear();
        self.screen = Screen::Panel;
        self.refresh_panel()
    }

    /// Plugin, permissions et fonction de rendu du panneau ouvert.
    fn panel_context(&self) -> Option<(Plugin, std::collections::BTreeSet<Permission>, String)> {
        let entry = self.panels.get(self.active_panel?)?;
        let plugin = self
            .plugins
            .iter()
            .find(|p| p.plugin.manifest.id == entry.plugin_id)?;
        let granted = self.granted(plugin)?;
        Some((plugin.plugin.clone(), granted, entry.callback.clone()))
    }

    /// Redessine le panneau : le plugin reçoit l'état courant et renvoie ses composants.
    fn refresh_panel(&mut self) -> Task<Message> {
        let Some((plugin, granted, callback)) = self.panel_context() else {
            return Task::none();
        };
        let input = self.host_input(&plugin, &granted);
        match host::render_panel(&plugin, &granted, input, &callback, &self.panel_state) {
            Ok((widgets, effects)) => {
                // Les valeurs proposées par le plugin ne servent que tant que l'utilisateur n'a rien changé.
                for widget in &widgets {
                    if let (Some(id), Some(value)) = (widget.id(), widget.default_value()) {
                        self.panel_state.entry(id.to_string()).or_insert(value);
                    }
                }
                self.panel_widgets = widgets;
                self.apply_effects(&plugin.manifest.id, effects, false)
            }
            Err(error) => {
                self.status = format!(
                    "{}: {} ({error})",
                    self.t("plugin_error"),
                    plugin.manifest.name
                );
                Task::none()
            }
        }
    }

    fn panel_call(&mut self, function: &str, args: Vec<Dynamic>) -> Task<Message> {
        let Some((plugin, granted, _)) = self.panel_context() else {
            return Task::none();
        };
        let input = self.host_input(&plugin, &granted);
        match host::run(&plugin, &granted, input, function, args) {
            Ok(outcome) => self.apply_effects(&plugin.manifest.id, outcome.effects, false),
            Err(error) => {
                self.status = format!(
                    "{}: {} ({error})",
                    self.t("plugin_error"),
                    plugin.manifest.name
                );
                Task::none()
            }
        }
    }

    fn panel_changed(&mut self, id: String, value: PanelValue) -> Task<Message> {
        // Seuls les composants réellement affichés peuvent être modifiés.
        let Some(callback) = self
            .panel_widgets
            .iter()
            .find(|w| w.id() == Some(id.as_str()))
            .map(|w| w.on_change().map(str::to_string))
        else {
            return Task::none();
        };
        self.panel_state.insert(id, value.clone());
        let mut tasks = Vec::new();
        if let Some(callback) = callback {
            let args = vec![
                value.to_dynamic(),
                host::state_map(&self.panel_state).into(),
            ];
            tasks.push(self.panel_call(&callback, args));
        }
        tasks.push(self.refresh_panel());
        Task::batch(tasks)
    }

    fn panel_button(&mut self, callback: String, arg: String) -> Task<Message> {
        let shown = self.panel_widgets.iter().any(|w| {
            matches!(w, Widget::Button { callback: c, arg: a, .. } if *c == callback && *a == arg)
        });
        if !shown {
            return Task::none();
        }
        let args = vec![
            Dynamic::from(arg),
            host::state_map(&self.panel_state).into(),
        ];
        let call = self.panel_call(&callback, args);
        Task::batch([call, self.refresh_panel()])
    }

    fn approve(&mut self, id: &str) {
        if let Some(entry) = self.plugins.iter().find(|p| p.plugin.manifest.id == id) {
            self.settings
                .plugins
                .approved
                .insert(id.to_string(), entry.plugin.hash.clone());
        }
    }

    fn install_plugin(&mut self, folder: &Path) {
        let plugin = match Plugin::load_dir(folder) {
            Ok(plugin) => plugin,
            Err(errors) => {
                self.status = format!(
                    "{}: {}",
                    self.t("plugin_rejected"),
                    errors.first().cloned().unwrap_or_default()
                );
                return;
            }
        };
        if plugin::builtin::load_all()
            .iter()
            .any(|p| p.manifest.id == plugin.manifest.id)
        {
            self.status = format!("{}: {}", self.t("plugin_rejected"), plugin.manifest.id);
            return;
        }
        let target = self.paths.plugins.join(&plugin.manifest.id);
        let manifest_text = std::fs::read_to_string(folder.join("plugin.toml")).unwrap_or_default();
        let written = std::fs::create_dir_all(&target)
            .and_then(|_| std::fs::write(target.join("plugin.toml"), manifest_text))
            .and_then(|_| std::fs::write(target.join(&plugin.manifest.entry), &plugin.source));
        match written {
            Ok(()) => {
                self.reload_plugins();
                self.status = self.t("plugin_installed");
            }
            Err(error) => self.status = error.to_string(),
        }
    }

    // --- Mise à jour ------------------------------------------------------------------------------------

    pub fn update(&mut self, message: Message) -> Task<Message> {
        let task = self.handle(message);
        // Hors du panneau, l'aperçu direct s'arrête et le panneau est fermé.
        if self.screen != Screen::Panel {
            self.live_theme = None;
            self.active_panel = None;
        } else if self.active_panel.is_none() {
            self.screen = Screen::Home;
            self.live_theme = None;
        }
        task
    }

    fn handle(&mut self, message: Message) -> Task<Message> {
        if !matches!(message, Message::Tick | Message::Poll | Message::SyncTick) {
            self.touch();
        }
        // Pendant une opération sur le coffre, tout ce qui écrit dans la base ou verrouille est refusé :
        // une note écrite avec l'ancienne clé pendant le rechiffrement deviendrait illisible.
        if self.busy
            && matches!(
                message,
                Message::SaveNote
                    | Message::AskDelete(_)
                    | Message::ConfirmDelete(_)
                    | Message::PluginCall(_)
                    | Message::PanelChanged(..)
                    | Message::PanelButton(..)
                    | Message::SyncNow
                    | Message::SyncTick
                    | Message::PullConfig(_)
                    | Message::PushConfig
                    | Message::ImportBackup
                    | Message::ExportBackup
                    | Message::LockNow
                    | Message::EnableVault
                    | Message::DisableVault
                    | Message::ChangeVault
            )
        {
            self.status = self.t("busy");
            return Task::none();
        }
        match message {
            Message::NewNote => self.start_new_note(),
            Message::SeeNotes => {
                // Les notes sont déjà chargées au démarrage et après chaque sauvegarde.
                self.status.clear();
                self.screen = Screen::See;
            }
            Message::BackHome => {
                self.status.clear();
                self.preview = None;
                self.screen = Screen::Home;
            }
            Message::OpenSettings => {
                self.status.clear();
                self.preview = None;
                self.screen = Screen::Settings;
            }
            Message::OpenScreen(screen) => {
                self.status.clear();
                self.screen = screen;
            }
            Message::ToggleTheme(dark) => {
                self.settings.dark_mode = dark;
                self.settings.theme.id = if dark {
                    "default-dark"
                } else {
                    "default-light"
                }
                .to_string();
                self.settings.theme.accent = None;
                self.accent_input.clear();
                self.update_settings();
            }
            Message::TitleChanged(value) => {
                self.title = value;
                self.status.clear();
            }
            Message::BodyEdited(action) => {
                self.body.perform(action);
                self.status.clear();
            }
            Message::SaveNote => {
                // Un second clic (ou Ctrl+S) pendant un enregistrement ne doit pas créer un doublon.
                if self.io_pending > 0 {
                    return Task::none();
                }
                if let Some(request) = self.prepare_save() {
                    return self.save_task(request);
                }
            }
            Message::SaveFinished(result) => return self.finish_save(result),
            Message::SaveSettings => {
                self.update_settings();
                self.status = self.t("settings_saved");
            }
            Message::LanguageChanged(language) => {
                self.settings.language = language;
                self.update_settings();
            }
            Message::ToggleTimestamps(value) => {
                self.settings.show_timestamps = value;
                self.update_settings();
            }
            Message::ToggleCompact(value) => {
                self.settings.compact_mode = value;
                self.update_settings();
            }
            Message::ToggleConfirmDelete(value) => {
                self.settings.confirm_delete = value;
                self.update_settings();
            }
            // Pas d'écriture disque pendant le glissement : voir FontSizeReleased.
            Message::FontSizeChanged(value) => self.set_font_size(value),
            Message::FontSizeReleased => self.update_settings(),
            Message::BackFromWrite => {
                self.status.clear();
                self.screen = if self.is_editing() {
                    Screen::See
                } else {
                    Screen::Home
                };
            }
            Message::SearchChanged(value) => self.search = value,
            Message::ToggleExpand(uid) => {
                if !self.expanded.remove(&uid) {
                    self.expanded.insert(uid);
                }
            }
            Message::ToggleMenu(uid) => {
                self.pending_delete = None;
                self.menu_open = if self.menu_open.as_deref() == Some(&uid) {
                    None
                } else {
                    Some(uid)
                };
            }
            Message::EditNote(uid) => self.open_note(&uid),
            Message::CopyNote(uid) => {
                self.menu_open = None;
                if let Some(text) = self.note_text(&uid) {
                    self.status = self.t("copied");
                    return iced::clipboard::write(text);
                }
            }
            Message::AskDelete(uid) => {
                self.menu_open = None;
                if self.settings.confirm_delete {
                    self.pending_delete = Some(uid);
                } else {
                    return self.delete_task(uid);
                }
            }
            Message::ConfirmDelete(uid) => return self.delete_task(uid),
            Message::CancelDelete => self.pending_delete = None,
            Message::DeleteFinished(result) => return self.finish_delete(result),

            Message::UnlockPasswordChanged(value) => self.unlock_password = value,
            Message::Unlock => return self.start_unlock(),
            Message::UnlockFinished(result) => self.finish_unlock(result),
            Message::LockNow => self.lock(),
            Message::PickKeyfile(target) => return Self::pick_keyfile(target),
            Message::KeyfilePicked(target, result) => match result {
                Ok(Some(file)) => match target {
                    KeyfileTarget::Unlock => self.unlock_keyfile = Some(file),
                    KeyfileTarget::Security => self.form.keyfile = Some(file),
                },
                Ok(None) => {}
                Err(error) => self.status = error,
            },
            Message::ClearKeyfile(target) => match target {
                KeyfileTarget::Unlock => self.unlock_keyfile = None,
                KeyfileTarget::Security => self.form.keyfile = None,
            },
            Message::SecField(field, value) => {
                match field {
                    SecField::Password => self.form.password = value,
                    SecField::Confirm => self.form.confirm = value,
                    SecField::Current => self.form.current = value,
                    SecField::Backup => self.form.backup = value,
                }
                self.status.clear();
            }
            Message::SecCipher(cipher) => self.form.cipher = cipher,
            Message::SecKdf(preset) => self.form.preset = preset,
            Message::AutoLock(minutes) => {
                self.settings.security.auto_lock_minutes = minutes;
                self.persist_settings();
            }
            Message::GeneratePassword => {
                let generated = crypto::generate_password(24);
                self.form.password = generated.clone();
                self.form.confirm = generated;
            }
            Message::EnableVault => return self.start_enable_vault(),
            Message::DisableVault => return self.start_disable_vault(),
            Message::ChangeVault => return self.start_change_vault(),
            Message::VaultFinished(result) => self.finish_vault(result),
            Message::ExportBackup => return self.export_backup(),
            Message::ImportBackup => return self.import_backup(),
            Message::ImportFinished(result) => match result {
                Ok((key, settings)) => {
                    self.status = self.t(&key);
                    self.form.backup.clear();
                    if let Some(settings) = settings {
                        self.apply_backup_appearance(*settings);
                    }
                    if !self.locked {
                        self.load_notes();
                    }
                    return self.sync_after_change();
                }
                Err(error) if error.is_empty() => {}
                Err(error) => {
                    self.status = if error.contains("wrong password") {
                        self.t("wrong_password")
                    } else {
                        error
                    }
                }
            },
            Message::BackupFinished(result) => match result {
                Ok(key) => {
                    self.status = self.t(&key);
                    self.form.backup.clear();
                    if !self.locked {
                        self.load_notes();
                    }
                }
                Err(error) if error.is_empty() => {}
                Err(error) => {
                    self.status = if error.contains("wrong password") {
                        self.t("wrong_password")
                    } else {
                        error
                    }
                }
            },

            Message::PreviewTheme(id) => self.preview = Some(id),
            Message::CancelPreview => self.preview = None,
            Message::ApplyTheme(id) => self.apply_theme(id),
            Message::ThemeFilterChanged(filter) => self.theme_filter = filter,
            Message::ThemeSearch(value) => self.theme_search = value,
            Message::AccentInput(value) => {
                self.accent_input = value.clone();
                if value.trim().is_empty() {
                    self.settings.theme.accent = None;
                    self.persist_settings();
                } else if let Some(color) = theme::parse_hex(&value) {
                    self.settings.theme.accent = Some(color.to_hex());
                    self.persist_settings();
                }
            }
            Message::RadiusChanged(v) => {
                self.settings.theme.radius = v;
                self.settings.theme.clamp();
            }
            Message::BorderChanged(v) => {
                self.settings.theme.border_width = v;
                self.settings.theme.clamp();
            }
            Message::SpacingChanged(v) => {
                self.settings.theme.spacing = v;
                self.settings.theme.clamp();
            }
            Message::PanelContrastChanged(v) => {
                self.settings.theme.panel_contrast = v;
                self.settings.theme.clamp();
            }
            Message::ThemeSliderReleased => self.persist_settings(),
            Message::ToggleEditorMono(v) => {
                self.settings.theme.editor_monospace = v;
                self.persist_settings();
            }
            Message::ToggleUiMono(v) => {
                self.settings.theme.ui_monospace = v;
                self.persist_settings();
            }
            Message::CustomNameChanged(value) => self.custom_name = value,
            Message::SaveCustomTheme => self.save_custom_theme(),

            Message::SyncToggle(v) => {
                self.settings.sync.enabled = v;
                self.persist_settings();
                self.ensure_listener();
            }
            Message::SyncName(v) => {
                self.settings.sync.device_name = v.chars().take(40).collect();
                self.persist_settings();
            }
            Message::SyncPrimary(v) => {
                self.settings.sync.is_primary = v;
                self.persist_settings();
            }
            Message::SyncListen(v) => {
                self.settings.sync.listen = v;
                self.persist_settings();
                self.ensure_listener();
            }
            Message::SyncPort(text) => {
                self.port_input = text.clone();
                if let Ok(port) = text.trim().parse::<u16>() {
                    if port >= 1024 {
                        self.settings.sync.port = port;
                        self.persist_settings();
                        self.ensure_listener();
                    }
                }
            }
            Message::SyncTrusted(v) => {
                self.settings.sync.trusted_networks_only = v;
                self.persist_settings();
            }
            Message::SyncSecret(v) => {
                self.settings.sync.secret = v;
                self.persist_settings();
                self.ensure_listener();
            }
            Message::SyncGenerateSecret => {
                self.settings.sync.secret = crypto::generate_password(32);
                self.show_secret = true;
                self.persist_settings();
                self.ensure_listener();
            }
            Message::SyncToggleSecret => self.show_secret = !self.show_secret,
            Message::PeerInput(v) => self.peer_input = v,
            Message::AddPeer => {
                // Sans port explicite, on prend celui de ce PC ; les adresses invalides sont refusées.
                match sync::normalize_peer(&self.peer_input, self.settings.sync.port) {
                    Some(peer) if self.settings.sync.peers.len() < 64 => {
                        if !self.settings.sync.peers.contains(&peer) {
                            self.settings.sync.peers.push(peer);
                            self.persist_settings();
                        }
                        self.peer_input.clear();
                    }
                    _ => self.status = self.t("invalid_peer"),
                }
            }
            Message::RemovePeer(index) => {
                if index < self.settings.sync.peers.len() {
                    self.settings.sync.peers.remove(index);
                    self.persist_settings();
                }
            }
            Message::SyncModeChanged(mode) => {
                self.settings.sync.mode = mode;
                self.persist_settings();
            }
            Message::SyncInterval(v) => {
                self.settings.sync.interval_secs = v.clamp(10, 86_400);
                self.persist_settings();
            }
            Message::SyncEncrypt(v) => {
                self.settings.sync.encrypt_transport = v;
                self.persist_settings();
            }
            Message::SyncOnSave(v) => {
                self.settings.sync.sync_on_save = v;
                self.persist_settings();
            }
            Message::SyncNow | Message::SyncTick => return self.start_sync(),
            Message::PullConfig(index) => return self.start_pull_config(index),
            Message::PushConfig => return self.start_push_config(),
            Message::SyncOpFinished(result) => self.finish_sync_op(result),
            Message::Poll => self.poll_events(),

            Message::PluginEnable(id) => {
                self.approve(&id);
                self.persist_settings();
                self.reload_plugins();
            }
            Message::PluginDisable(id) => {
                self.settings.plugins.approved.remove(&id);
                self.persist_settings();
                self.reload_plugins();
            }
            Message::PluginEnableBuiltin => {
                let ids: Vec<String> = self
                    .plugins
                    .iter()
                    .filter(|p| p.plugin.builtin)
                    .map(|p| p.plugin.manifest.id.clone())
                    .collect();
                for id in ids {
                    self.approve(&id);
                }
                self.persist_settings();
                self.reload_plugins();
            }
            Message::PluginInstall => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .pick_folder()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    Message::PluginInstallPicked,
                );
            }
            Message::PluginInstallPicked(folder) => {
                if let Some(folder) = folder {
                    self.install_plugin(&folder);
                }
            }
            Message::PluginRemove(id) => {
                let removable = self
                    .plugins
                    .iter()
                    .any(|p| p.plugin.manifest.id == id && !p.plugin.builtin);
                if removable {
                    let _ = std::fs::remove_dir_all(self.paths.plugins.join(&id));
                    self.settings.plugins.approved.remove(&id);
                    self.persist_settings();
                    self.reload_plugins();
                }
            }
            Message::PluginCall(call) => return self.call_plugin(call),

            Message::OpenPanel(index) => return self.open_panel(index),
            Message::ClosePanel => {
                self.status.clear();
                self.screen = Screen::Home;
            }
            Message::PanelChanged(id, value) => return self.panel_changed(id, value),
            Message::PanelButton(callback, arg) => return self.panel_button(callback, arg),
            Message::Tick => {
                let minutes = self.settings.security.auto_lock_minutes;
                if self.vault.is_some()
                    && minutes > 0
                    && self.last_activity.elapsed() >= Duration::from_secs(minutes as u64 * 60)
                {
                    return self.auto_lock();
                }
            }
        }
        Task::none()
    }
}
