//! API exposée aux scripts. Chaque fonction vérifie à l'exécution que la permission correspondante a
//! été accordée (en plus de l'analyse statique) et se contente d'enregistrer un `Effect` : le script ne
//! modifie jamais l'application directement.

use super::host::{Effect, HostInput, Zone, MAX_EFFECTS, MAX_TEXT_BYTES};
use super::Permission;
use rhai::{Array, Dynamic, Engine, EvalAltResult, ImmutableString};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

type Res<T> = Result<T, Box<EvalAltResult>>;

/// Toutes les fonctions de l'API et la permission qu'elles exigent (`None` : libre d'accès).
pub const API: &[(&str, Option<Permission>)] = &[
    ("notes_list", Some(Permission::NotesRead)),
    ("notes_get", Some(Permission::NotesRead)),
    ("notes_search", Some(Permission::NotesRead)),
    ("notes_count", Some(Permission::NotesRead)),
    ("notes_add", Some(Permission::NotesWrite)),
    ("notes_update", Some(Permission::NotesWrite)),
    ("notes_delete", Some(Permission::NotesDelete)),
    ("editor_title", Some(Permission::EditorRead)),
    ("editor_text", Some(Permission::EditorRead)),
    ("editor_set_title", Some(Permission::EditorWrite)),
    ("editor_set_text", Some(Permission::EditorWrite)),
    ("editor_insert", Some(Permission::EditorWrite)),
    ("clipboard_set", Some(Permission::ClipboardWrite)),
    ("ui_button", Some(Permission::UiButton)),
    ("ui_note_action", Some(Permission::UiNoteAction)),
    ("ui_status", Some(Permission::UiStatus)),
    ("theme_set", Some(Permission::ThemeSet)),
    ("theme_current", Some(Permission::SettingsRead)),
    ("setting_get", Some(Permission::SettingsRead)),
    ("storage_get", Some(Permission::Storage)),
    ("storage_set", Some(Permission::Storage)),
    ("now_iso", None),
    ("now_date", None),
    ("random_int", None),
    ("random_password", None),
    ("word_count", None),
];

pub fn permission_of(name: &str) -> Option<Permission> {
    API.iter().find(|(n, _)| *n == name).and_then(|(_, p)| *p)
}

#[derive(Clone)]
struct Ctx {
    input: Arc<HostInput>,
    granted: Arc<BTreeSet<Permission>>,
    effects: Arc<Mutex<Vec<Effect>>>,
}

impl Ctx {
    fn need(&self, permission: Permission) -> Res<()> {
        if self.granted.contains(&permission) {
            Ok(())
        } else {
            Err(format!("permission denied: {permission}").into())
        }
    }

    fn push(&self, effect: Effect) -> Res<()> {
        let mut effects = self.effects.lock().map_err(|_| "plugin state poisoned")?;
        if effects.len() >= MAX_EFFECTS {
            return Err("too many actions requested by the plugin".into());
        }
        effects.push(effect);
        Ok(())
    }
}

fn text(value: &ImmutableString) -> Res<String> {
    if value.len() > MAX_TEXT_BYTES {
        return Err("text too large".into());
    }
    Ok(value.to_string())
}

fn short(value: &ImmutableString, max: usize) -> Res<String> {
    if value.chars().count() > max {
        return Err(format!("text longer than {max} characters").into());
    }
    Ok(value.to_string())
}

/// Un nom de callback doit ressembler à un identifiant : il est ensuite appelé par l'application,
/// uniquement dans le script du même plugin.
fn callback_name(value: &ImmutableString) -> Res<String> {
    let ok = !value.is_empty()
        && value.len() <= 48
        && value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !value.starts_with(|c: char| c.is_ascii_digit());
    if ok {
        Ok(value.to_string())
    } else {
        Err("invalid callback name".into())
    }
}

pub fn register(
    engine: &mut Engine,
    granted: BTreeSet<Permission>,
    input: Arc<HostInput>,
    effects: Arc<Mutex<Vec<Effect>>>,
) {
    let ctx = Ctx {
        input,
        granted: Arc::new(granted),
        effects,
    };

    // --- Lecture des notes -------------------------------------------------------------------
    let c = ctx.clone();
    engine.register_fn("notes_list", move || -> Res<Array> {
        c.need(Permission::NotesRead)?;
        // Du plus récent au plus ancien, comme dans l'application.
        Ok(c.input
            .notes
            .iter()
            .rev()
            .map(|n| Dynamic::from(n.to_map()))
            .collect())
    });
    let c = ctx.clone();
    engine.register_fn("notes_count", move || -> Res<i64> {
        c.need(Permission::NotesRead)?;
        Ok(c.input.notes.len() as i64)
    });
    let c = ctx.clone();
    engine.register_fn("notes_get", move |id: ImmutableString| -> Res<Dynamic> {
        c.need(Permission::NotesRead)?;
        Ok(c.input
            .notes
            .iter()
            .find(|n| n.uid == id.as_str())
            .map(|n| Dynamic::from(n.to_map()))
            .unwrap_or(Dynamic::UNIT))
    });
    let c = ctx.clone();
    engine.register_fn(
        "notes_search",
        move |query: ImmutableString| -> Res<Array> {
            c.need(Permission::NotesRead)?;
            let query = query.to_lowercase();
            Ok(c.input
                .notes
                .iter()
                .rev()
                .filter(|n| {
                    n.title.to_lowercase().contains(&query)
                        || n.body.to_lowercase().contains(&query)
                })
                .map(|n| Dynamic::from(n.to_map()))
                .collect())
        },
    );

    // --- Écriture des notes ------------------------------------------------------------------
    let c = ctx.clone();
    engine.register_fn(
        "notes_add",
        move |title: ImmutableString, body: ImmutableString| -> Res<()> {
            c.need(Permission::NotesWrite)?;
            c.push(Effect::AddNote {
                title: short(&title, 300)?,
                body: text(&body)?,
            })
        },
    );
    let c = ctx.clone();
    engine.register_fn(
        "notes_update",
        move |id: ImmutableString, title: ImmutableString, body: ImmutableString| -> Res<()> {
            c.need(Permission::NotesWrite)?;
            c.push(Effect::UpdateNote {
                uid: short(&id, 64)?,
                title: short(&title, 300)?,
                body: text(&body)?,
            })
        },
    );
    let c = ctx.clone();
    engine.register_fn("notes_delete", move |id: ImmutableString| -> Res<()> {
        c.need(Permission::NotesDelete)?;
        c.push(Effect::DeleteNote(short(&id, 64)?))
    });

    // --- Éditeur -----------------------------------------------------------------------------
    let c = ctx.clone();
    engine.register_fn("editor_title", move || -> Res<String> {
        c.need(Permission::EditorRead)?;
        Ok(c.input.editor_title.clone())
    });
    let c = ctx.clone();
    engine.register_fn("editor_text", move || -> Res<String> {
        c.need(Permission::EditorRead)?;
        Ok(c.input.editor_text.clone())
    });
    let c = ctx.clone();
    engine.register_fn(
        "editor_set_title",
        move |value: ImmutableString| -> Res<()> {
            c.need(Permission::EditorWrite)?;
            c.push(Effect::EditorSetTitle(short(&value, 300)?))
        },
    );
    let c = ctx.clone();
    engine.register_fn(
        "editor_set_text",
        move |value: ImmutableString| -> Res<()> {
            c.need(Permission::EditorWrite)?;
            c.push(Effect::EditorSetText(text(&value)?))
        },
    );
    let c = ctx.clone();
    engine.register_fn("editor_insert", move |value: ImmutableString| -> Res<()> {
        c.need(Permission::EditorWrite)?;
        c.push(Effect::EditorInsert(text(&value)?))
    });

    // --- Interface ---------------------------------------------------------------------------
    let c = ctx.clone();
    engine.register_fn("clipboard_set", move |value: ImmutableString| -> Res<()> {
        c.need(Permission::ClipboardWrite)?;
        c.push(Effect::Clipboard(text(&value)?))
    });
    let c = ctx.clone();
    engine.register_fn("ui_status", move |value: ImmutableString| -> Res<()> {
        c.need(Permission::UiStatus)?;
        c.push(Effect::Status(short(&value, 200)?))
    });
    let c = ctx.clone();
    engine.register_fn(
        "ui_button",
        move |zone: ImmutableString,
              label: ImmutableString,
              callback: ImmutableString|
              -> Res<()> {
            c.need(Permission::UiButton)?;
            let zone = Zone::parse(&zone).ok_or("zone must be \"home\", \"notes\" or \"write\"")?;
            c.push(Effect::Button {
                zone,
                label: short(&label, 40)?,
                callback: callback_name(&callback)?,
            })
        },
    );
    let c = ctx.clone();
    engine.register_fn(
        "ui_note_action",
        move |label: ImmutableString, callback: ImmutableString| -> Res<()> {
            c.need(Permission::UiNoteAction)?;
            c.push(Effect::NoteAction {
                label: short(&label, 40)?,
                callback: callback_name(&callback)?,
            })
        },
    );
    let c = ctx.clone();
    engine.register_fn("theme_set", move |id: ImmutableString| -> Res<()> {
        c.need(Permission::ThemeSet)?;
        c.push(Effect::SetTheme(short(&id, 64)?))
    });
    let c = ctx.clone();
    engine.register_fn("theme_current", move || -> Res<String> {
        c.need(Permission::SettingsRead)?;
        Ok(c.input.theme_id.clone())
    });
    let c = ctx.clone();
    engine.register_fn("setting_get", move |key: ImmutableString| -> Res<Dynamic> {
        c.need(Permission::SettingsRead)?;
        Ok(c.input
            .settings
            .get(key.as_str())
            .map(|v| Dynamic::from(v.clone()))
            .unwrap_or(Dynamic::UNIT))
    });

    // --- Stockage privé du plugin ------------------------------------------------------------
    let c = ctx.clone();
    engine.register_fn("storage_get", move |key: ImmutableString| -> Res<Dynamic> {
        c.need(Permission::Storage)?;
        Ok(c.input
            .storage
            .get(key.as_str())
            .map(|v| Dynamic::from(v.clone()))
            .unwrap_or(Dynamic::UNIT))
    });
    let c = ctx.clone();
    engine.register_fn(
        "storage_set",
        move |key: ImmutableString, value: ImmutableString| -> Res<()> {
            c.need(Permission::Storage)?;
            let valid = !key.is_empty() && key.len() <= 64 && value.len() <= 64 * 1024;
            if !valid {
                return Err("invalid storage key or value".into());
            }
            c.push(Effect::StorageSet {
                key: key.to_string(),
                value: value.to_string(),
            })
        },
    );

    // --- Fonctions libres : sans effet sur les données de l'utilisateur ---------------------------
    engine.register_fn("now_iso", || chrono::Local::now().to_rfc3339());
    engine.register_fn("now_date", || {
        chrono::Local::now().format("%Y-%m-%d").to_string()
    });
    engine.register_fn("random_int", |low: i64, high: i64| -> i64 {
        let (low, high) = if low <= high {
            (low, high)
        } else {
            (high, low)
        };
        let span = (high as i128 - low as i128 + 1) as u128;
        let random = u64::from_le_bytes(crate::crypto::random_bytes::<8>()) as u128;
        (low as i128 + (random % span) as i128) as i64
    });
    engine.register_fn("random_password", |len: i64| -> String {
        crate::crypto::generate_password(len.clamp(12, 128) as usize)
    });
    engine.register_fn("word_count", |value: ImmutableString| -> i64 {
        value.split_whitespace().count() as i64
    });
}
