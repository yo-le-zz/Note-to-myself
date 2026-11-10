//! Système de plugins.
//!
//! Un plugin est un dossier `plugin.toml` + un script Rhai (`main.rhai`). Rhai est un moteur de script
//! embarqué qui n'a, par construction, aucun accès au disque, au réseau ni aux processus : un plugin ne
//! peut faire que ce que l'API de `api.rs` lui expose, et cette API n'a aucune fonction réseau.
//!
//! Les permissions ne sont **jamais** lues dans le manifeste : elles sont déduites du code par
//! `analyze::analyze`, présentées à l'utilisateur, puis appliquées à l'exécution.

pub mod analyze;
pub mod api;
pub mod builtin;
pub mod host;

use serde::Deserialize;
use std::collections::BTreeSet;
use std::fmt;

/// Taille maximale d'un script de plugin.
pub const MAX_SOURCE_BYTES: usize = 128 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Permission {
    NotesRead,
    NotesWrite,
    NotesDelete,
    EditorRead,
    EditorWrite,
    ClipboardWrite,
    UiButton,
    UiNoteAction,
    UiStatus,
    ThemeSet,
    SettingsRead,
    Storage,
    HookStart,
    HookNoteSave,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Risk {
    Low,
    Medium,
    High,
}

impl Permission {
    pub fn id(self) -> &'static str {
        match self {
            Self::NotesRead => "notes.read",
            Self::NotesWrite => "notes.write",
            Self::NotesDelete => "notes.delete",
            Self::EditorRead => "editor.read",
            Self::EditorWrite => "editor.write",
            Self::ClipboardWrite => "clipboard.write",
            Self::UiButton => "ui.button",
            Self::UiNoteAction => "ui.note_action",
            Self::UiStatus => "ui.status",
            Self::ThemeSet => "theme.set",
            Self::SettingsRead => "settings.read",
            Self::Storage => "storage",
            Self::HookStart => "hook.start",
            Self::HookNoteSave => "hook.note_save",
        }
    }

    pub fn describe(self) -> &'static str {
        match self {
            Self::NotesRead => "Read the content of all your notes",
            Self::NotesWrite => "Create and modify notes",
            Self::NotesDelete => "Delete notes",
            Self::EditorRead => "Read the note being edited",
            Self::EditorWrite => "Change the note being edited",
            Self::ClipboardWrite => "Put text on the clipboard",
            Self::UiButton => "Add buttons to the interface",
            Self::UiNoteAction => "Add actions to the note menu",
            Self::UiStatus => "Show status messages",
            Self::ThemeSet => "Change the app theme",
            Self::SettingsRead => "Read app settings (language, font size…)",
            Self::Storage => "Keep its own small private data store",
            Self::HookStart => "Run when the app starts",
            Self::HookNoteSave => "Modify every note just before it is saved",
        }
    }

    pub fn risk(self) -> Risk {
        match self {
            Self::UiButton
            | Self::UiNoteAction
            | Self::UiStatus
            | Self::ThemeSet
            | Self::SettingsRead
            | Self::Storage
            | Self::HookStart => Risk::Low,
            Self::EditorRead | Self::EditorWrite | Self::ClipboardWrite | Self::NotesRead => {
                Risk::Medium
            }
            Self::NotesWrite | Self::NotesDelete | Self::HookNoteSave => Risk::High,
        }
    }
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

/// Fonctions que le script peut définir et que l'application appelle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Hook {
    Start,
    NoteSave,
}

impl Hook {
    pub fn function_name(self) -> &'static str {
        match self {
            Self::Start => "on_start",
            Self::NoteSave => "on_note_save",
        }
    }

    pub fn arity(self) -> usize {
        match self {
            Self::Start => 0,
            Self::NoteSave => 1,
        }
    }

    pub fn permission(self) -> Permission {
        match self {
            Self::Start => Permission::HookStart,
            Self::NoteSave => Permission::HookNoteSave,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Analysis {
    pub permissions: BTreeSet<Permission>,
    pub hooks: BTreeSet<Hook>,
    /// Avertissements affichés à l'utilisateur (combinaisons sensibles, permissions ignorées du manifeste…).
    pub warnings: Vec<String>,
}

impl Analysis {
    pub fn risk(&self) -> Risk {
        self.permissions
            .iter()
            .map(|p| p.risk())
            .max()
            .unwrap_or(Risk::Low)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_entry")]
    pub entry: String,
}

fn default_entry() -> String {
    "main.rhai".to_string()
}

impl Manifest {
    pub fn parse(text: &str) -> Result<Manifest, String> {
        let manifest: Manifest = toml::from_str(text).map_err(|e| format!("plugin.toml: {e}"))?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<(), String> {
        let id_ok = !self.id.is_empty()
            && self.id.len() <= 48
            && self
                .id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            && !self.id.starts_with('-');
        if !id_ok {
            return Err("plugin.toml: `id` must be 1-48 chars of a-z, 0-9 and '-'".into());
        }
        if self.name.trim().is_empty() || self.name.len() > 60 {
            return Err("plugin.toml: `name` must be 1-60 characters".into());
        }
        if self.version.trim().is_empty() || self.version.len() > 20 {
            return Err("plugin.toml: `version` is required".into());
        }
        if self.description.len() > 300 || self.author.len() > 60 {
            return Err("plugin.toml: `description` (300) or `author` (60) too long".into());
        }
        // Le point d'entrée reste dans le dossier du plugin.
        let entry_ok = !self.entry.is_empty()
            && self.entry.ends_with(".rhai")
            && !self.entry.contains(['/', '\\', ':'])
            && !self.entry.contains("..");
        if !entry_ok {
            return Err(
                "plugin.toml: `entry` must be a .rhai file name inside the plugin folder".into(),
            );
        }
        Ok(())
    }

    /// Le manifeste contient-il une clé `permissions` ? Elle est ignorée, mais signalée.
    pub fn declares_permissions(text: &str) -> bool {
        text.parse::<toml::Table>()
            .map(|t| t.contains_key("permissions"))
            .unwrap_or(false)
    }
}

/// Plugin chargé et analysé, prêt à être approuvé puis exécuté.
#[derive(Debug, Clone)]
pub struct Plugin {
    pub manifest: Manifest,
    pub source: String,
    /// Empreinte SHA-256 du manifeste + du script : si elle change, l'approbation est à refaire.
    pub hash: String,
    pub analysis: Analysis,
    pub builtin: bool,
}

impl Plugin {
    pub fn load(manifest_text: &str, source: &str, builtin: bool) -> Result<Plugin, Vec<String>> {
        let manifest = Manifest::parse(manifest_text).map_err(|e| vec![e])?;
        if source.len() > MAX_SOURCE_BYTES {
            return Err(vec![format!(
                "script is larger than {} KiB",
                MAX_SOURCE_BYTES / 1024
            )]);
        }
        let mut analysis = analyze::analyze(source)?;
        if Manifest::declares_permissions(manifest_text) {
            analysis.warnings.push(
                "`permissions` in plugin.toml is ignored: permissions are detected from the code"
                    .into(),
            );
        }
        let mut data = manifest_text.as_bytes().to_vec();
        data.push(0);
        data.extend_from_slice(source.as_bytes());
        let hash = crate::crypto::to_hex(&crate::crypto::sha256(&data));
        Ok(Plugin {
            manifest,
            source: source.to_string(),
            hash,
            analysis,
            builtin,
        })
    }

    /// Charge un plugin depuis un dossier du disque.
    pub fn load_dir(dir: &std::path::Path) -> Result<Plugin, Vec<String>> {
        let read = |name: &str| -> Result<String, Vec<String>> {
            let path = dir.join(name);
            let meta = std::fs::metadata(&path).map_err(|e| vec![format!("{name}: {e}")])?;
            if meta.len() as usize > MAX_SOURCE_BYTES {
                return Err(vec![format!("{name}: file too large")]);
            }
            std::fs::read_to_string(&path).map_err(|e| vec![format!("{name}: {e}")])
        };
        let manifest_text = read("plugin.toml")?;
        let manifest = Manifest::parse(&manifest_text).map_err(|e| vec![e])?;
        let source = read(&manifest.entry)?;
        Plugin::load(&manifest_text, &source, false)
    }
}
