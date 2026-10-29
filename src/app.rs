use directories::ProjectDirs;
use iced::widget::text_editor;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Screen {
    Home,
    Write,
    See,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    English,
    French,
    Spanish,
    German,
    Italian,
    Portuguese,
    Dutch,
    Polish,
    Russian,
    Ukrainian,
    Turkish,
    Arabic,
    Japanese,
    Korean,
    Chinese,
    Hindi,
    Swedish,
    Greek,
    Romanian,
    Czech,
}

impl Language {
    pub const ALL: [Language; 20] = [
        Self::English, Self::French, Self::Spanish, Self::German, Self::Italian,
        Self::Portuguese, Self::Dutch, Self::Polish, Self::Russian, Self::Ukrainian,
        Self::Turkish, Self::Arabic, Self::Japanese, Self::Korean, Self::Chinese,
        Self::Hindi, Self::Swedish, Self::Greek, Self::Romanian, Self::Czech,
    ];
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::English => "English",
            Self::French => "Français",
            Self::Spanish => "Español",
            Self::German => "Deutsch",
            Self::Italian => "Italiano",
            Self::Portuguese => "Português",
            Self::Dutch => "Nederlands",
            Self::Polish => "Polski",
            Self::Russian => "Русский",
            Self::Ukrainian => "Українська",
            Self::Turkish => "Türkçe",
            Self::Arabic => "العربية",
            Self::Japanese => "日本語",
            Self::Korean => "한국어",
            Self::Chinese => "中文",
            Self::Hindi => "हिन्दी",
            Self::Swedish => "Svenska",
            Self::Greek => "Ελληνικά",
            Self::Romanian => "Română",
            Self::Czech => "Čeština",
        };
        write!(f, "{name}")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub dark_mode: bool,
    pub language: Language,
    pub show_timestamps: bool,
    pub compact_mode: bool,
    pub confirm_delete: bool,
    pub font_size: u16,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            dark_mode: true,
            language: Language::English,
            show_timestamps: true,
            compact_mode: false,
            confirm_delete: true,
            font_size: 15,
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Note {
    pub id: i64,
    /// Numéro d'affichage : 1 pour la plus ancienne note, stable tant qu'on ne supprime pas.
    pub number: usize,
    pub title: String,
    pub body: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct SaveRequest {
    database_path: PathBuf,
    title: String,
    body: String,
    editing_id: Option<i64>,
    now: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    NewNote,
    SeeNotes,
    BackHome,
    OpenSettings,
    ToggleTheme(bool),
    TitleChanged(String),
    BodyEdited(text_editor::Action),
    SaveNote,
    SaveFinished(Result<Vec<Note>, String>),
    SaveSettings,
    LanguageChanged(Language),
    ToggleTimestamps(bool),
    ToggleCompact(bool),
    ToggleConfirmDelete(bool),
    FontSizeChanged(u16),
    FontSizeReleased,
    BackFromWrite,
    SearchChanged(String),
    ToggleExpand(i64),
    ToggleMenu(i64),
    EditNote(i64),
    CopyNote(i64),
    AskDelete(i64),
    ConfirmDelete(i64),
    CancelDelete,
    DeleteFinished(Result<Vec<Note>, String>),
}

pub struct App {
    pub settings: Settings,
    pub screen: Screen,
    pub title: String,
    pub body: text_editor::Content,
    pub status: String,
    pub notes: Vec<Note>,
    pub search: String,
    pub expanded: HashSet<i64>,
    pub menu_open: Option<i64>,
    pub pending_delete: Option<i64>,
    settings_path: PathBuf,
    database_path: PathBuf,
    editing_id: Option<i64>,
}

impl App {
    pub fn new() -> Self {
        let dirs = ProjectDirs::from("com", "NoteToMyself", "NoteToMyself");
        let data_dir = dirs
            .as_ref()
            .map(|d| d.data_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        let config_dir = dirs
            .as_ref()
            .map(|d| d.config_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

        let _ = fs::create_dir_all(&data_dir);
        let _ = fs::create_dir_all(&config_dir);

        let settings_path = config_dir.join("settings.json");
        let database_path = data_dir.join("notes.sqlite3");

        let mut settings = fs::read_to_string(&settings_path)
            .ok()
            .and_then(|json| serde_json::from_str::<Settings>(&json).ok())
            .unwrap_or_default();
        // Sécurise aussi les anciennes préférences déjà enregistrées.
        settings.font_size = settings.font_size.clamp(12, 24);

        let mut app = Self {
            settings,
            screen: Screen::Home,
            title: String::new(),
            body: text_editor::Content::new(),
            status: String::new(),
            notes: Vec::new(),
            search: String::new(),
            expanded: HashSet::new(),
            menu_open: None,
            pending_delete: None,
            settings_path,
            database_path,
            editing_id: None,
        };

        app.initialize_database();
        app.load_notes();
        app
    }

    fn connection(&self) -> rusqlite::Result<Connection> {
        let connection = Connection::open(&self.database_path)?;
        // Évite une attente SQLite longue si un autre processus tient le verrou.
        let _ = connection.busy_timeout(Duration::from_millis(150));
        Ok(connection)
    }

    fn initialize_database(&self) {
        if let Ok(connection) = self.connection() {
            let _ = Self::ensure_notes_schema(&connection);
        }
    }

    fn table_columns(connection: &Connection) -> rusqlite::Result<Vec<String>> {
        let mut statement = connection.prepare("PRAGMA table_info(notes)")?;
        let columns = statement
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(columns)
    }

    /// Crée la table si nécessaire et la met au schéma actuel sans perdre de données.
    ///
    /// Les anciennes versions stockaient le texte dans `content`, `text`, `note`…
    /// Ces colonnes pouvaient être `NOT NULL` sans valeur par défaut : un simple
    /// `ALTER TABLE ADD COLUMN body` ne les supprime pas, et chaque INSERT échouait
    /// avec « NOT NULL constraint failed: notes.content ». Dans ce cas, la table est
    /// reconstruite proprement ; l'ancienne est conservée sous `notes_legacy_backup`.
    fn ensure_notes_schema(connection: &Connection) -> rusqlite::Result<()> {
        const CURRENT: [&str; 5] = ["id", "title", "body", "created_at", "updated_at"];
        const LEGACY_TEXT: [&str; 5] = ["content", "text", "note", "message", "description"];

        connection.execute(
            "CREATE TABLE IF NOT EXISTS notes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL DEFAULT '',
                body TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL DEFAULT '',
                updated_at TEXT NOT NULL DEFAULT ''
            )",
            [],
        )?;

        let columns = Self::table_columns(connection)?;
        let up_to_date = columns.len() == CURRENT.len()
            && CURRENT.iter().all(|c| columns.iter().any(|name| name == c));
        if up_to_date {
            return Ok(());
        }

        let has = |name: &str| columns.iter().any(|c| c == name);

        // Texte de la note : `body` s'il est rempli, sinon l'ancienne colonne.
        let mut body_sources: Vec<String> = Vec::new();
        for name in std::iter::once("body").chain(LEGACY_TEXT) {
            if has(name) {
                body_sources.push(format!("NULLIF(CAST(\"{name}\" AS TEXT), '')"));
            }
        }
        body_sources.push("''".to_string());
        let body_expr = format!("COALESCE({})", body_sources.join(", "));

        let title_expr = if has("title") { "COALESCE(CAST(title AS TEXT), '')" } else { "''" };
        let created_expr = if has("created_at") {
            "COALESCE(CAST(created_at AS TEXT), '')"
        } else {
            "''"
        };
        let updated_expr = if has("updated_at") && has("created_at") {
            "COALESCE(NULLIF(CAST(updated_at AS TEXT), ''), CAST(created_at AS TEXT), '')"
        } else if has("updated_at") {
            "COALESCE(CAST(updated_at AS TEXT), '')"
        } else if has("created_at") {
            "COALESCE(CAST(created_at AS TEXT), '')"
        } else {
            "''"
        };

        let (id_col, id_expr) = if has("id") { ("id, ", "id, ") } else { ("", "") };

        let transaction = connection.unchecked_transaction()?;
        transaction.execute("DROP TABLE IF EXISTS notes_new", [])?;
        transaction.execute(
            "CREATE TABLE notes_new (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL DEFAULT '',
                body TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL DEFAULT '',
                updated_at TEXT NOT NULL DEFAULT ''
            )",
            [],
        )?;
        transaction.execute(
            &format!(
                "INSERT INTO notes_new ({id_col}title, body, created_at, updated_at)
                 SELECT {id_expr}{title_expr}, {body_expr}, {created_expr}, {updated_expr}
                 FROM notes"
            ),
            [],
        )?;
        transaction.execute("DROP TABLE IF EXISTS notes_legacy_backup", [])?;
        transaction.execute("ALTER TABLE notes RENAME TO notes_legacy_backup", [])?;
        transaction.execute("ALTER TABLE notes_new RENAME TO notes", [])?;
        transaction.commit()
    }

    fn persist_settings(&self) {
        if let Ok(json) = serde_json::to_string_pretty(&self.settings) {
            let _ = fs::write(&self.settings_path, json);
        }
    }

    /// Lit toutes les notes, la plus récente en premier. Le numéro d'une note
    /// est son rang de création (1 = la plus ancienne).
    fn query_notes(connection: &Connection) -> rusqlite::Result<Vec<Note>> {
        let mut statement = connection.prepare(
            "SELECT id, title, body, created_at, updated_at FROM notes ORDER BY id ASC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(Note {
                id: row.get(0)?,
                number: 0,
                title: row.get(1)?,
                body: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;
        let mut notes = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        for (index, note) in notes.iter_mut().enumerate() {
            note.number = index + 1;
        }
        notes.reverse();
        Ok(notes)
    }

    pub fn load_notes(&mut self) {
        let Ok(connection) = self.connection() else {
            self.status = self.t("database_error");
            return;
        };
        match Self::query_notes(&connection) {
            Ok(notes) => self.notes = notes,
            Err(_) => self.status = self.t("read_error"),
        }
    }

    pub fn start_new_note(&mut self) {
        self.title.clear();
        self.body = text_editor::Content::new();
        self.editing_id = None;
        self.status.clear();
        self.screen = Screen::Write;
    }

    /// Prépare une sauvegarde sans bloquer l'interface. La requête SQLite
    /// sera exécutée par un worker lancé depuis `main.rs`.
    pub fn prepare_save(&mut self) -> Option<SaveRequest> {
        let title = self.title.trim().to_owned();
        let body = self.body.text();

        if title.is_empty() && body.trim().is_empty() {
            self.status = self.t("empty_note");
            return None;
        }

        self.status = self.t("saving");
        Some(SaveRequest {
            database_path: self.database_path.clone(),
            title,
            body,
            editing_id: self.editing_id,
            now: chrono::Local::now().to_rfc3339(),
        })
    }

    /// Fonction appelée sur un thread de travail : écriture puis lecture
    /// des notes, afin de ne pas refaire une requête SQLite sur le thread UI.
    pub fn save_request(request: SaveRequest) -> Result<Vec<Note>, String> {
        let connection = Connection::open(&request.database_path)
            .map_err(|error| format!("database open failed: {error}"))?;
        connection
            .busy_timeout(Duration::from_millis(500))
            .map_err(|error| format!("database timeout setup failed: {error}"))?;

        Self::ensure_notes_schema(&connection)
            .map_err(|error| format!("database schema migration failed: {error}"))?;

        if let Some(id) = request.editing_id {
            let changed = connection
                .execute(
                    "UPDATE notes SET title = ?1, body = ?2, updated_at = ?3 WHERE id = ?4",
                    params![request.title, request.body, request.now, id],
                )
                .map_err(|error| format!("note update failed: {error}"))?;
            if changed == 0 {
                return Err("the note to update no longer exists".into());
            }
        } else {
            connection
                .execute(
                    "INSERT INTO notes (title, body, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?3)",
                    params![request.title, request.body, request.now],
                )
                .map_err(|error| format!("note insert failed: {error}"))?;
        }

        Self::query_notes(&connection).map_err(|error| format!("reading saved notes failed: {error}"))
    }

    /// Suppression exécutée hors du thread d'interface.
    pub fn delete_request(database_path: PathBuf, id: i64) -> Result<Vec<Note>, String> {
        let connection = Connection::open(&database_path)
            .map_err(|error| format!("database open failed: {error}"))?;
        connection
            .busy_timeout(Duration::from_millis(500))
            .map_err(|error| format!("database timeout setup failed: {error}"))?;
        connection
            .execute("DELETE FROM notes WHERE id = ?1", [id])
            .map_err(|error| format!("note delete failed: {error}"))?;
        Self::query_notes(&connection).map_err(|error| format!("reading notes failed: {error}"))
    }

    pub fn database_path(&self) -> PathBuf {
        self.database_path.clone()
    }

    pub fn finish_save(&mut self, result: Result<Vec<Note>, String>) {
        match result {
            Ok(notes) => {
                self.notes = notes;
                self.title.clear();
                self.body = text_editor::Content::new();
                self.editing_id = None;
                self.status = self.t("saved");
                self.screen = Screen::See;
            }
            Err(error) => {
                self.status = format!("{} ({error})", self.t("save_error"));
            }
        }
    }

    pub fn t(&self, key: &str) -> String {
        crate::i18n::tr(self.settings.language, key).to_string()
    }

    pub fn open_note(&mut self, id: i64) {
        if let Some(note) = self.notes.iter().find(|note| note.id == id) {
            self.title = note.title.clone();
            self.body = text_editor::Content::with_text(&note.body);
            self.editing_id = Some(note.id);
            self.status.clear();
            self.menu_open = None;
            self.screen = Screen::Write;
        }
    }

    pub fn is_editing(&self) -> bool {
        self.editing_id.is_some()
    }

    pub fn note_text(&self, id: i64) -> Option<String> {
        self.notes.iter().find(|n| n.id == id).map(|n| {
            if n.title.trim().is_empty() {
                n.body.clone()
            } else {
                format!("{}\n\n{}", n.title, n.body)
            }
        })
    }

    pub fn finish_delete(&mut self, result: Result<Vec<Note>, String>) {
        self.pending_delete = None;
        match result {
            Ok(notes) => {
                self.expanded.retain(|id| notes.iter().any(|n| n.id == *id));
                self.notes = notes;
                self.status = self.t("deleted");
            }
            Err(error) => self.status = format!("{} ({error})", self.t("delete_error")),
        }
    }

    pub fn update_settings(&mut self) {
        self.set_font_size(self.settings.font_size);
        self.persist_settings();
    }

    /// Change la taille du texte sans toucher au disque (utilisé pendant le
    /// glissement du curseur, qui émet des dizaines d'événements par seconde).
    pub fn set_font_size(&mut self, size: u16) {
        self.settings.font_size = size.clamp(12, 24);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_legacy_not_null_content_column() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE notes (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    content TEXT NOT NULL,
                    created_at TEXT NOT NULL
                );
                INSERT INTO notes (content, created_at) VALUES ('ancienne note', '2025-01-01');",
            )
            .unwrap();

        App::ensure_notes_schema(&connection).unwrap();

        // Un INSERT avec le schéma actuel ne doit plus échouer.
        connection
            .execute(
                "INSERT INTO notes (title, body, created_at, updated_at) VALUES ('t', 'b', 'x', 'x')",
                [],
            )
            .unwrap();
        let old: String = connection
            .query_row("SELECT body FROM notes WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(old, "ancienne note");
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM notes", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2);

        // Idempotent.
        App::ensure_notes_schema(&connection).unwrap();
    }

    #[test]
    fn fresh_database_is_untouched() {
        let connection = Connection::open_in_memory().unwrap();
        App::ensure_notes_schema(&connection).unwrap();
        App::ensure_notes_schema(&connection).unwrap();
        assert_eq!(App::table_columns(&connection).unwrap().len(), 5);
    }
}
