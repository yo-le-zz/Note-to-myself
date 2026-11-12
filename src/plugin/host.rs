//! Exécution d'un plugin : données visibles par le script, effets demandés à l'application.
//!
//! Le script ne touche jamais directement à l'état de l'application : il lit un instantané
//! (`HostInput`) et produit une liste d'effets (`Effect`) que l'application applique elle-même.

use super::panel::{self, PanelValue, Widget};
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

/// Un thème tel que le voit un plugin (couleurs en `#rrggbb`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeInfo {
    pub id: String,
    pub name: String,
    pub dark: bool,
    pub bg: String,
    pub fg: String,
    pub accent: String,
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
    pub themes: Vec<ThemeInfo>,
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
    /// Enregistre un panneau (au démarrage seulement).
    Panel {
        title: String,
        callback: String,
    },
    /// Change la valeur d'un composant du panneau ouvert.
    PanelSet {
        id: String,
        value: PanelValue,
    },
    /// Aperçu en direct d'une palette, sans rien enregistrer.
    ThemePreview {
        dark: bool,
        bg: String,
        fg: String,
        accent: String,
    },
    ThemePreviewClear,
    ThemeSave {
        name: String,
        dark: bool,
        bg: String,
        fg: String,
        accent: String,
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
    // Par défaut Rhai exécuterait d'abord les instructions du niveau global du script, à chaque appel :
    // un plugin n'a pas à fonctionner ainsi, seules ses fonctions sont appelées par l'application.
    let options = rhai::CallFnOptions::new()
        .eval_ast(false)
        .rewind_scope(true);
    let value: Dynamic = engine
        .call_fn_with_options(options, &mut scope, &ast, function, args)
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

/// Appelle la fonction de rendu d'un panneau avec l'état courant de ses composants.
pub fn render_panel(
    plugin: &Plugin,
    granted: &BTreeSet<Permission>,
    input: HostInput,
    callback: &str,
    state: &BTreeMap<String, PanelValue>,
) -> Result<(Vec<Widget>, Vec<Effect>), String> {
    let outcome = run(
        plugin,
        granted,
        input,
        callback,
        vec![state_map(state).into()],
    )?;
    let widgets = panel::parse_widgets(outcome.value.unwrap_or(Dynamic::UNIT))?;
    Ok((widgets, outcome.effects))
}

/// L'état d'un panneau sous la forme transmise aux fonctions du plugin.
pub fn state_map(state: &BTreeMap<String, PanelValue>) -> Map {
    let mut map = Map::new();
    for (key, value) in state {
        map.insert(key.as_str().into(), value.to_dynamic());
    }
    map
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

#[cfg(test)]
mod tests {
    use super::*;

    fn plugin(source: &str) -> Plugin {
        Plugin::load("id = \"t\"\nname = \"t\"\nversion = \"1\"\n", source, false).unwrap()
    }

    #[test]
    fn global_statements_never_run_when_a_function_is_called() {
        let p = plugin("ui_status(\"top-level\");\nfn act() { ui_status(\"inside\"); }");
        let granted = p.analysis.permissions.clone();
        let out = run(&p, &granted, HostInput::default(), "act", vec![]).unwrap();
        assert_eq!(out.effects, vec![Effect::Status("inside".into())]);
    }

    #[test]
    fn infinite_loops_and_runaway_recursion_are_stopped() {
        let looping = plugin("fn act() { let i = 0; while true { i += 1; } }");
        let err = run(
            &looping,
            &BTreeSet::new(),
            HostInput::default(),
            "act",
            vec![],
        )
        .unwrap_err();
        assert!(!err.is_empty());
        let recursive = plugin("fn act() { act() }");
        assert!(run(
            &recursive,
            &BTreeSet::new(),
            HostInput::default(),
            "act",
            vec![]
        )
        .is_err());
        let hungry = plugin("fn act() { let s = \"aaaaaaaaaa\"; loop { s += s; } }");
        assert!(run(
            &hungry,
            &BTreeSet::new(),
            HostInput::default(),
            "act",
            vec![]
        )
        .is_err());
    }

    #[test]
    fn a_plugin_cannot_flood_the_application_with_actions() {
        let p = plugin("fn act() { for i in 0..1000 { ui_status(\"x\"); } }");
        let granted = p.analysis.permissions.clone();
        let err = run(&p, &granted, HostInput::default(), "act", vec![]).unwrap_err();
        assert!(err.contains("too many actions"), "{err}");
    }

    #[test]
    fn invalid_button_requests_are_rejected_by_the_api() {
        for call in [
            "ui_button(\"anywhere\", \"L\", \"cb\")",
            "ui_button(\"home\", \"L\", \"not a name\")",
            "ui_button(\"home\", \"L\", \"1abc\")",
            "ui_button(\"home\", \"L\", \"\")",
        ] {
            let p = plugin(&format!("fn act() {{ {call}; }}"));
            let granted = p.analysis.permissions.clone();
            assert!(
                run(&p, &granted, HostInput::default(), "act", vec![]).is_err(),
                "{call}"
            );
        }
    }
}
