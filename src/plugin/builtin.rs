//! Plugins livrés avec l'application (dossiers `plugins/` du dépôt, embarqués à la compilation).

use super::Plugin;

const SOURCES: &[(&str, &str)] = &[
    (
        include_str!("../../plugins/word-counter/plugin.toml"),
        include_str!("../../plugins/word-counter/main.rhai"),
    ),
    (
        include_str!("../../plugins/insert-date/plugin.toml"),
        include_str!("../../plugins/insert-date/main.rhai"),
    ),
    (
        include_str!("../../plugins/auto-trim/plugin.toml"),
        include_str!("../../plugins/auto-trim/main.rhai"),
    ),
    (
        include_str!("../../plugins/title-from-first-line/plugin.toml"),
        include_str!("../../plugins/title-from-first-line/main.rhai"),
    ),
    (
        include_str!("../../plugins/daily-note/plugin.toml"),
        include_str!("../../plugins/daily-note/main.rhai"),
    ),
    (
        include_str!("../../plugins/checklist/plugin.toml"),
        include_str!("../../plugins/checklist/main.rhai"),
    ),
    (
        include_str!("../../plugins/random-note/plugin.toml"),
        include_str!("../../plugins/random-note/main.rhai"),
    ),
    (
        include_str!("../../plugins/duplicate-note/plugin.toml"),
        include_str!("../../plugins/duplicate-note/main.rhai"),
    ),
    (
        include_str!("../../plugins/emoji-bar/plugin.toml"),
        include_str!("../../plugins/emoji-bar/main.rhai"),
    ),
    (
        include_str!("../../plugins/todo-tagger/plugin.toml"),
        include_str!("../../plugins/todo-tagger/main.rhai"),
    ),
    (
        include_str!("../../plugins/lorem-ipsum/plugin.toml"),
        include_str!("../../plugins/lorem-ipsum/main.rhai"),
    ),
    (
        include_str!("../../plugins/password-generator/plugin.toml"),
        include_str!("../../plugins/password-generator/main.rhai"),
    ),
    (
        include_str!("../../plugins/reading-time/plugin.toml"),
        include_str!("../../plugins/reading-time/main.rhai"),
    ),
    (
        include_str!("../../plugins/clean-empty-notes/plugin.toml"),
        include_str!("../../plugins/clean-empty-notes/main.rhai"),
    ),
    (
        include_str!("../../plugins/copy-last-note/plugin.toml"),
        include_str!("../../plugins/copy-last-note/main.rhai"),
    ),
    (
        include_str!("../../plugins/smart-typography/plugin.toml"),
        include_str!("../../plugins/smart-typography/main.rhai"),
    ),
    (
        include_str!("../../plugins/quick-theme/plugin.toml"),
        include_str!("../../plugins/quick-theme/main.rhai"),
    ),
    (
        include_str!("../../plugins/startup-count/plugin.toml"),
        include_str!("../../plugins/startup-count/main.rhai"),
    ),
];

/// Plugins intégrés. Un plugin intégré invalide est une erreur de développement : les tests la détectent,
/// et l'application l'ignore plutôt que de s'arrêter.
pub fn load_all() -> Vec<Plugin> {
    SOURCES
        .iter()
        .filter_map(|(manifest, source)| Plugin::load(manifest, source, true).ok())
        .collect()
}

pub fn count() -> usize {
    SOURCES.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::host::{self, HostInput, NoteView};
    use crate::plugin::{Hook, Permission};
    use std::collections::BTreeSet;

    fn granted(plugin: &Plugin) -> BTreeSet<Permission> {
        plugin.analysis.permissions.clone()
    }

    fn find(id: &str) -> Plugin {
        load_all()
            .into_iter()
            .find(|p| p.manifest.id == id)
            .unwrap_or_else(|| panic!("missing {id}"))
    }

    fn sample_input() -> HostInput {
        let note = |uid: &str, title: &str, body: &str| NoteView {
            uid: uid.into(),
            title: title.into(),
            body: body.into(),
            created_at: "2026-01-01T00:00:00+00:00".into(),
            updated_at: "2026-01-01T00:00:00+00:00".into(),
        };
        HostInput {
            notes: vec![
                note("a", "First", "one two three"),
                note("b", "", ""),
                note("c", "Third", "four five"),
            ],
            editor_title: "Draft".into(),
            editor_text: "alpha beta gamma delta".into(),
            ..Default::default()
        }
    }

    #[test]
    fn at_least_fifteen_builtin_plugins_all_valid() {
        assert!(count() >= 15 && count() == 18);
        let loaded = load_all();
        assert_eq!(loaded.len(), count(), "a built-in plugin failed to load");
        let ids: BTreeSet<_> = loaded.iter().map(|p| p.manifest.id.clone()).collect();
        assert_eq!(ids.len(), loaded.len());
        for plugin in &loaded {
            assert_eq!(plugin.manifest.author, "yolezz");
            assert!(plugin.builtin);
            assert!(
                !plugin.analysis.permissions.is_empty(),
                "{}",
                plugin.manifest.id
            );
        }
    }

    #[test]
    fn every_start_hook_runs_cleanly() {
        for plugin in load_all() {
            if plugin.analysis.hooks.contains(&Hook::Start) {
                let effects = host::run_start(&plugin, &granted(&plugin), sample_input())
                    .unwrap_or_else(|e| panic!("{}: {e}", plugin.manifest.id));
                assert!(
                    !effects.is_empty(),
                    "{} did nothing at start",
                    plugin.manifest.id
                );
            }
        }
    }

    #[test]
    fn every_requested_callback_exists_and_runs() {
        use crate::plugin::host::Effect;
        for plugin in load_all() {
            let effects =
                host::run_start(&plugin, &granted(&plugin), sample_input()).unwrap_or_default();
            for effect in effects {
                let (name, arity, args) = match effect {
                    Effect::Button { callback, .. } => (callback, 0, vec![]),
                    Effect::NoteAction { callback, .. } => {
                        let note = sample_input().notes[0].to_map();
                        (callback, 1, vec![note.into()])
                    }
                    _ => continue,
                };
                assert!(
                    host::defines(&plugin, &name, arity),
                    "{}: missing fn {name}",
                    plugin.manifest.id
                );
                host::run(&plugin, &granted(&plugin), sample_input(), &name, args)
                    .unwrap_or_else(|e| panic!("{}::{name}: {e}", plugin.manifest.id));
            }
        }
    }

    #[test]
    fn auto_trim_cleans_whitespace_and_blank_runs() {
        let plugin = find("auto-trim");
        let (_, body, _) = host::run_note_save(
            &plugin,
            &granted(&plugin),
            HostInput::default(),
            "t",
            "a  \n\n\n\n\nb\t\n\n",
        )
        .unwrap();
        assert_eq!(body, "a\n\n\nb");
    }

    #[test]
    fn title_from_first_line_only_fills_empty_titles() {
        let plugin = find("title-from-first-line");
        let g = granted(&plugin);
        let (title, _, _) = host::run_note_save(
            &plugin,
            &g,
            HostInput::default(),
            "",
            "\n  Hello world\nrest",
        )
        .unwrap();
        assert_eq!(title, "Hello world");
        let (kept, _, _) =
            host::run_note_save(&plugin, &g, HostInput::default(), "Mine", "other").unwrap();
        assert_eq!(kept, "Mine");
        let long = "x".repeat(100);
        let (cut, _, _) =
            host::run_note_save(&plugin, &g, HostInput::default(), "", &long).unwrap();
        assert_eq!(cut.chars().count(), 60);
    }

    #[test]
    fn todo_tagger_and_typography() {
        let plugin = find("todo-tagger");
        let g = granted(&plugin);
        let (title, _, _) =
            host::run_note_save(&plugin, &g, HostInput::default(), "Shopping", "TODO milk")
                .unwrap();
        assert_eq!(title, "[TODO] Shopping");
        let (again, _, _) =
            host::run_note_save(&plugin, &g, HostInput::default(), &title, "TODO milk").unwrap();
        assert_eq!(again, "[TODO] Shopping");
        let typo = find("smart-typography");
        let (_, body, _) = host::run_note_save(
            &typo,
            &granted(&typo),
            HostInput::default(),
            "t",
            "wait... a -- b",
        )
        .unwrap();
        assert_eq!(body, "wait… a — b");
    }

    #[test]
    fn word_counter_reports_totals() {
        use crate::plugin::host::Effect;
        let plugin = find("word-counter");
        let out = host::run(
            &plugin,
            &granted(&plugin),
            sample_input(),
            "show_stats",
            vec![],
        )
        .unwrap();
        assert_eq!(
            out.effects,
            vec![Effect::Status("3 notes, 5 words, 22 characters".into())]
        );
    }

    #[test]
    fn clean_empty_notes_deletes_only_empty_ones() {
        use crate::plugin::host::Effect;
        let plugin = find("clean-empty-notes");
        let out = host::run(&plugin, &granted(&plugin), sample_input(), "clean", vec![]).unwrap();
        assert!(out.effects.contains(&Effect::DeleteNote("b".into())));
        assert!(!out.effects.contains(&Effect::DeleteNote("a".into())));
    }

    #[test]
    fn permissions_are_enforced_at_runtime() {
        let plugin = find("word-counter");
        // Sans la permission de lecture des notes, l'appel échoue : l'analyse n'est pas la seule barrière.
        let none = BTreeSet::new();
        let result = host::run(&plugin, &none, sample_input(), "show_stats", vec![]);
        assert!(result.unwrap_err().contains("permission denied"));
    }

    #[test]
    fn copy_last_note_is_flagged_as_sensitive() {
        let plugin = find("copy-last-note");
        assert!(!plugin.analysis.warnings.is_empty());
    }
}
