//! Analyse statique d'un plugin : déduit les permissions réellement utilisées par le code et
//! refuse les constructions interdites.
//!
//! L'analyse lit le flux de jetons du script : tout identifiant égal au nom d'une fonction de l'API
//! compte comme un usage. C'est volontairement prudent (une variable qui porte le nom d'une fonction
//! de l'API est comptée), jamais l'inverse. Comme `Fn`, `call`, `curry` et `eval` sont interdits, il est
//! impossible de construire un nom de fonction à l'exécution pour contourner cette analyse.

use super::{api, Analysis, Hook, Permission};
use rhai::{Engine, LexError, OptimizationLevel, Token};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

/// Identifiants interdits dans un plugin : ils permettraient d'appeler une fonction dont le nom est
/// calculé, d'évaluer du texte comme du code ou d'introspecter le script.
pub const FORBIDDEN: [&str; 9] = [
    "eval",
    "Fn",
    "call",
    "curry",
    "is_def_fn",
    "is_def_var",
    "global",
    "import",
    "export",
];

/// Crée un moteur Rhai bridé. Si `identifiers` est fourni, il reçoit tous les identifiants rencontrés.
pub fn sandbox_engine(identifiers: Option<Arc<Mutex<BTreeSet<String>>>>) -> Engine {
    let mut engine = Engine::new();
    // L'optimiseur pourrait replier des appels : on veut analyser et exécuter le code tel qu'écrit.
    engine.set_optimization_level(OptimizationLevel::None);
    engine.set_max_operations(500_000);
    engine.set_max_call_levels(32);
    engine.set_max_expr_depths(64, 32);
    engine.set_max_string_size(super::host::MAX_TEXT_BYTES);
    engine.set_max_array_size(50_000);
    engine.set_max_map_size(512);
    engine.set_max_variables(256);
    engine.set_max_functions(64);
    // `print` et `debug` ne doivent rien écrire sur la sortie de l'application.
    engine.on_print(|_| {});
    engine.on_debug(|_, _, _| {});
    for symbol in FORBIDDEN {
        engine.disable_symbol(symbol);
    }
    #[allow(deprecated)]
    engine.on_parse_token(move |token, _pos, _state| {
        if let Token::Identifier(name) = &token {
            let name: &str = name;
            if FORBIDDEN.contains(&name) {
                return Token::LexError(Box::new(LexError::ImproperSymbol(
                    name.to_string(),
                    format!("`{name}` is not allowed in plugins"),
                )));
            }
            if let Some(found) = &identifiers {
                if let Ok(mut set) = found.lock() {
                    set.insert(name.to_string());
                }
            }
        }
        token
    });
    engine
}

/// Analyse un script. Retourne les permissions déduites, ou la liste des problèmes qui le rendent refusé.
pub fn analyze(source: &str) -> Result<Analysis, Vec<String>> {
    if source.len() > super::MAX_SOURCE_BYTES {
        return Err(vec!["script is too large".into()]);
    }
    let identifiers = Arc::new(Mutex::new(BTreeSet::new()));
    let engine = sandbox_engine(Some(identifiers.clone()));
    let ast = engine
        .compile(source)
        .map_err(|e| vec![format!("syntax: {e}")])?;

    let mut analysis = Analysis::default();
    let used = identifiers
        .lock()
        .map_err(|_| vec!["analysis failed".to_string()])?
        .clone();
    for name in &used {
        if let Some(permission) = api::permission_of(name) {
            analysis.permissions.insert(permission);
        }
    }

    let mut errors = Vec::new();
    for hook in [Hook::Start, Hook::NoteSave] {
        let defined: Vec<usize> = ast
            .iter_functions()
            .filter(|f| f.name == hook.function_name())
            .map(|f| f.params.len())
            .collect();
        if defined.is_empty() {
            continue;
        }
        if defined.contains(&hook.arity()) {
            analysis.hooks.insert(hook);
            analysis.permissions.insert(hook.permission());
        } else {
            errors.push(format!(
                "`{}` must take exactly {} argument(s)",
                hook.function_name(),
                hook.arity()
            ));
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    let reads = analysis.permissions.contains(&Permission::NotesRead)
        || analysis.permissions.contains(&Permission::EditorRead);
    if reads && analysis.permissions.contains(&Permission::ClipboardWrite) {
        analysis
            .warnings
            .push("Can copy the content of your notes to the clipboard".into());
    }
    if analysis.permissions.contains(&Permission::HookNoteSave) {
        analysis
            .warnings
            .push("Runs on every note you save and can rewrite it".into());
    }
    Ok(analysis)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permissions_come_from_the_code() {
        let a = analyze(r#"let n = notes_count(); ui_status("n = " + n);"#).unwrap();
        assert_eq!(
            a.permissions,
            BTreeSet::from([Permission::NotesRead, Permission::UiStatus])
        );
        assert!(a.warnings.is_empty());
    }

    #[test]
    fn a_pure_script_needs_no_permission() {
        let a = analyze("fn double(x) { x * 2 } let y = double(4);").unwrap();
        assert!(a.permissions.is_empty());
        assert_eq!(a.risk(), super::super::Risk::Low);
    }

    #[test]
    fn names_inside_strings_and_comments_do_not_count() {
        let a = analyze("// notes_delete(\"x\")\nlet s = \"notes_delete\"; let n = 1;").unwrap();
        assert!(a.permissions.is_empty());
    }

    #[test]
    fn method_call_syntax_is_still_detected() {
        let a = analyze(r#"let x = ""; x.clipboard_set();"#);
        // `clipboard_set` appelée en méthode : détectée comme un appel direct.
        assert!(a.unwrap().permissions.contains(&Permission::ClipboardWrite));
    }

    #[test]
    fn calls_inside_functions_and_branches_are_found() {
        let a = analyze(
            r#"fn cleanup() { if true { for i in 0..3 { notes_delete("x"); } } } fn other() { editor_insert("a"); }"#,
        )
        .unwrap();
        assert!(a.permissions.contains(&Permission::NotesDelete));
        assert!(a.permissions.contains(&Permission::EditorWrite));
        assert_eq!(a.risk(), super::super::Risk::High);
    }

    #[test]
    fn dynamic_dispatch_and_eval_are_rejected() {
        for source in [
            r#"eval("notes_delete(\"x\")");"#,
            r#"let f = Fn("notes_delete"); f.call("x");"#,
            r#"let f = |x| x; f.curry(1);"#,
            r#"call(f);"#,
            r#"is_def_fn("a", 1);"#,
            r#"import "x" as y;"#,
        ] {
            assert!(analyze(source).is_err(), "should be rejected: {source}");
        }
    }

    #[test]
    fn syntax_errors_are_reported() {
        assert!(analyze("let = ;").is_err());
        assert!(analyze("fn (").is_err());
    }

    #[test]
    fn oversized_scripts_are_rejected() {
        let big = "let a = 1;\n".repeat(super::super::MAX_SOURCE_BYTES / 8);
        assert!(analyze(&big).is_err());
    }

    #[test]
    fn hooks_are_detected_and_arity_is_checked() {
        let a = analyze("fn on_note_save(note) { note } fn on_start() { }").unwrap();
        assert!(a.hooks.contains(&Hook::NoteSave) && a.hooks.contains(&Hook::Start));
        assert!(a.permissions.contains(&Permission::HookNoteSave));
        assert!(!a.warnings.is_empty());
        assert!(analyze("fn on_note_save() { }").is_err());
        assert!(analyze("fn on_start(a) { }").is_err());
    }

    #[test]
    fn exfiltration_shaped_combination_is_flagged() {
        let a = analyze(r#"clipboard_set(notes_list().len().to_string());"#).unwrap();
        assert_eq!(a.warnings.len(), 1);
        assert!(a.warnings[0].contains("clipboard"));
    }

    #[test]
    fn there_is_no_network_or_file_api() {
        // Garantie structurelle : aucune fonction de l'API n'a de rapport avec le réseau ou les fichiers.
        for (name, _) in api::API {
            for banned in [
                "http", "net", "socket", "file", "fs_", "exec", "shell", "process", "env",
            ] {
                assert!(!name.contains(banned), "{name}");
            }
        }
    }
}
