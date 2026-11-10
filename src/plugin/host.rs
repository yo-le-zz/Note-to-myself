//! Exécution d'un plugin : données visibles par le script, effets demandés à l'application.
//!
//! Le script ne touche jamais directement à l'état de l'application : il lit un instantané
//! (`HostInput`) et produit une liste d'effets (`Effect`) que l'application applique elle-même.

use super::{analyze, api, Hook, Permission, Plugin};
use rhai::{Dynamic, Map, Scope, AST};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const MAX_EFFECTS: usize = 256;
pub const MAX_TEXT_BYTES: usize = 2 * 1024 * 1024;
const MAX_RUN_TIME: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteView {
    pub uid: String,
    pub title: String,
    pub body: String,
    pub created_at: String,
    pub updated_at: String,
}

impl NoteView {
    pub fn to_map(&self) -> Map {
        let mut map = Map::new();
        map.insert("id".into(), self.uid.clone().into());
        map.insert("title".into(), self.title.clone().into());
        map.insert("body".into(), self.body.clone().into());
        map.insert("created".into(), self.created_at.clone().into());
        map.insert("updated".into(), self.updated_at.clone().into());
        map
    }
}

/// Ce que le plugin a le droit de voir. Les champs non accordés sont simplement vides.
#[derive(Debug, Clone, Default)]
pub struct HostInput {
    pub notes: Vec<NoteView>,
    pub editor_title: String,
    pub editor_text: String,
    pub settings: BTreeMap<String, String>,
    pub theme_id: String,
    pub storage: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    Home,
    Notes,
    Write,
}

impl Zone {
    pub fn parse(text: &str) -> Option<Zone> {
        match text {
            "home" => Some(Zone::Home),
            "notes" => Some(Zone::Notes),
            "write" => Some(Zone::Write),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    Status(String),
    Clipboard(String),
    Button {
        zone: Zone,
        label: String,
        callback: String,
    },
    NoteAction {
        label: String,
        callback: String,
    },
    AddNote {
        title: String,
        body: String,
    },
    UpdateNote {
        uid: String,
        title: String,
        body: String,
    },
    DeleteNote(String),
    EditorSetTitle(String),
    EditorSetText(String),
    EditorInsert(String),
    SetTheme(String),
    StorageSet {
        key: String,
        value: String,
    },
}

#[derive(Debug, Default)]
pub struct Outcome {
    pub value: Option<Dynamic>,
    pub effects: Vec<Effect>,
}

/// Appelle `function` du plugin avec les permissions `granted`.
/// L'analyse a déjà eu lieu au chargement ; `granted` est ce que l'utilisateur a approuvé.
pub fn run(
    plugin: &Plugin,
    granted: &BTreeSet<Permission>,
    input: HostInput,
    function: &str,
    args: Vec<Dynamic>,
) -> Result<Outcome, String> {
    let effects = Arc::new(Mutex::new(Vec::new()));
    let mut engine = analyze::sandbox_engine(None);
    api::register(
        &mut engine,
        granted.clone(),
        Arc::new(input),
        effects.clone(),
    );

    let started = Instant::now();
    engine.on_progress(move |_| {
        (started.elapsed() > MAX_RUN_TIME).then(|| Dynamic::from("time limit exceeded"))
    });

    let ast: AST = engine.compile(&plugin.source).map_err(|e| e.to_string())?;
    let mut scope = Scope::new();
    let value: Dynamic = engine
        .call_fn(&mut scope, &ast, function, args)
        .map_err(|e| format!("{} ({function})", e))?;
    let effects = std::mem::take(
        &mut *effects
            .lock()
            .map_err(|_| "plugin state poisoned".to_string())?,
    );
    Ok(Outcome {
        value: Some(value),
        effects,
    })
}

/// Le plugin définit-il cette fonction avec ce nombre d'arguments ?
pub fn defines(plugin: &Plugin, name: &str, arity: usize) -> bool {
    let engine = analyze::sandbox_engine(None);
    engine
        .compile(&plugin.source)
        .map(|ast| {
            ast.iter_functions()
                .any(|f| f.name == name && f.params.len() == arity)
        })
        .unwrap_or(false)
}

/// Exécute `on_start` et retourne les effets demandés (boutons, statut…).
pub fn run_start(
    plugin: &Plugin,
    granted: &BTreeSet<Permission>,
    input: HostInput,
) -> Result<Vec<Effect>, String> {
    if !plugin.analysis.hooks.contains(&Hook::Start) || !granted.contains(&Permission::HookStart) {
        return Ok(Vec::new());
    }
    run(plugin, granted, input, Hook::Start.function_name(), vec![]).map(|o| o.effects)
}

/// Passe une note dans `on_note_save`. Retourne les champs éventuellement modifiés ; si le plugin échoue
/// ou renvoie autre chose qu'une table avec des textes, la note reste inchangée (elle n'est jamais perdue).
pub fn run_note_save(
    plugin: &Plugin,
    granted: &BTreeSet<Permission>,
    input: HostInput,
    title: &str,
    body: &str,
) -> Result<(String, String, Vec<Effect>), String> {
    if !plugin.analysis.hooks.contains(&Hook::NoteSave)
        || !granted.contains(&Permission::HookNoteSave)
    {
        return Ok((title.to_string(), body.to_string(), Vec::new()));
    }
    let mut note = Map::new();
    note.insert("title".into(), title.to_string().into());
    note.insert("body".into(), body.to_string().into());
    let outcome = run(
        plugin,
        granted,
        input,
        Hook::NoteSave.function_name(),
        vec![note.into()],
    )?;
    let mut new_title = title.to_string();
    let mut new_body = body.to_string();
    if let Some(map) = outcome.value.and_then(|v| v.try_cast::<Map>()) {
        if let Some(t) = map.get("title").and_then(|v| v.clone().into_string().ok()) {
            if t.len() <= MAX_TEXT_BYTES {
                new_title = t;
            }
        }
        if let Some(b) = map.get("body").and_then(|v| v.clone().into_string().ok()) {
            if b.len() <= MAX_TEXT_BYTES {
                new_body = b;
            }
        }
    }
    Ok((new_title, new_body, outcome.effects))
}
