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
    (
        include_str!("../../plugins/find-replace/plugin.toml"),
        include_str!("../../plugins/find-replace/main.rhai"),
    ),
    (
        include_str!("../../plugins/markdown-toolbar/plugin.toml"),
        include_str!("../../plugins/markdown-toolbar/main.rhai"),
    ),
    (
        include_str!("../../plugins/note-statistics/plugin.toml"),
        include_str!("../../plugins/note-statistics/main.rhai"),
    ),
    (
        include_str!("../../plugins/snippets/plugin.toml"),
        include_str!("../../plugins/snippets/main.rhai"),
    ),
    (
        include_str!("../../plugins/theme-scheduler/plugin.toml"),
        include_str!("../../plugins/theme-scheduler/main.rhai"),
    ),
    (
        include_str!("../../plugins/theme-studio/plugin.toml"),
        include_str!("../../plugins/theme-studio/main.rhai"),
    ),
    (
        include_str!("../../plugins/word-goal/plugin.toml"),
        include_str!("../../plugins/word-goal/main.rhai"),
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
        assert!(count() >= 15 && count() == 25);
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

    // --- Plugins à panneau --------------------------------------------------------------------

    use crate::plugin::host::{Effect as E, ThemeInfo};
    use crate::plugin::panel::{PanelValue, Widget};
    use rhai::Dynamic;
    use std::collections::BTreeMap;

    fn themed_input() -> HostInput {
        let theme =
            |id: &str, name: &str, dark: bool, bg: &str, fg: &str, accent: &str| ThemeInfo {
                id: id.into(),
                name: name.into(),
                dark,
                bg: bg.into(),
                fg: fg.into(),
                accent: accent.into(),
            };
        HostInput {
            theme_id: "default-dark".into(),
            themes: vec![
                theme(
                    "default-dark",
                    "Note Dark",
                    true,
                    "#0a0a0a",
                    "#ffffff",
                    "#d95229",
                ),
                theme(
                    "default-light",
                    "Note Light",
                    false,
                    "#ffffff",
                    "#0a0a0a",
                    "#d95229",
                ),
                theme("dracula", "Dracula", true, "#282a36", "#f8f8f2", "#bd93f9"),
            ],
            ..sample_input()
        }
    }

    fn state(pairs: &[(&str, PanelValue)]) -> BTreeMap<String, PanelValue> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    fn text(s: &str) -> PanelValue {
        PanelValue::Text(s.to_string())
    }

    /// Appelle une fonction de rappel d'un plugin avec ses permissions détectées.
    fn call(plugin: &Plugin, input: HostInput, function: &str, args: Vec<Dynamic>) -> Vec<E> {
        host::run(plugin, &granted(plugin), input, function, args)
            .unwrap_or_else(|e| panic!("{}::{function}: {e}", plugin.manifest.id))
            .effects
    }

    fn change(
        plugin: &Plugin,
        input: HostInput,
        function: &str,
        value: PanelValue,
        st: &BTreeMap<String, PanelValue>,
    ) -> Vec<E> {
        call(
            plugin,
            input,
            function,
            vec![value.to_dynamic(), host::state_map(st).into()],
        )
    }

    fn button(
        plugin: &Plugin,
        input: HostInput,
        function: &str,
        arg: &str,
        st: &BTreeMap<String, PanelValue>,
    ) -> Vec<E> {
        call(
            plugin,
            input,
            function,
            vec![Dynamic::from(arg.to_string()), host::state_map(st).into()],
        )
    }

    fn render(
        plugin: &Plugin,
        input: HostInput,
        callback: &str,
        st: &BTreeMap<String, PanelValue>,
    ) -> Vec<Widget> {
        host::render_panel(plugin, &granted(plugin), input, callback, st)
            .unwrap_or_else(|e| panic!("{}::{callback}: {e}", plugin.manifest.id))
            .0
    }

    fn seed(widgets: &[Widget]) -> BTreeMap<String, PanelValue> {
        widgets
            .iter()
            .filter_map(|w| Some((w.id()?.to_string(), w.default_value()?)))
            .collect()
    }

    fn texts(widgets: &[Widget]) -> Vec<String> {
        widgets
            .iter()
            .filter_map(|w| {
                if let Widget::Text(t) = w {
                    Some(t.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    #[test]
    fn every_panel_renders_and_all_its_callbacks_run() {
        let mut panels = 0;
        for plugin in load_all() {
            let effects =
                host::run_start(&plugin, &granted(&plugin), themed_input()).unwrap_or_default();
            for effect in effects {
                let E::Panel { callback, .. } = effect else {
                    continue;
                };
                panels += 1;
                let first = render(&plugin, themed_input(), &callback, &BTreeMap::new());
                assert!(!first.is_empty(), "{} renders nothing", plugin.manifest.id);
                let st = seed(&first);
                let second = render(&plugin, themed_input(), &callback, &st);
                assert!(!second.is_empty());
                for widget in second {
                    if let (Some(cb), Some(value)) = (widget.on_change(), widget.default_value()) {
                        change(&plugin, themed_input(), cb, value, &st);
                    }
                    if let Widget::Button { callback, arg, .. } = &widget {
                        button(&plugin, themed_input(), callback, arg, &st);
                    }
                }
            }
        }
        assert!(
            panels >= 5,
            "expected at least five panel plugins, found {panels}"
        );
    }

    #[test]
    fn theme_studio_has_its_own_ui_with_live_preview() {
        let plugin = find("theme-studio");
        assert!(plugin.analysis.permissions.contains(&Permission::UiPanel));
        assert!(plugin.analysis.permissions.contains(&Permission::ThemeEdit));
        assert_eq!(plugin.analysis.risk(), crate::plugin::Risk::Medium);

        let widgets = render(&plugin, themed_input(), "render_studio", &BTreeMap::new());
        assert!(widgets.iter().any(|w| matches!(w, Widget::Preview { .. })));
        let sliders = widgets
            .iter()
            .filter(|w| matches!(w, Widget::Slider { .. }))
            .count();
        assert!(sliders >= 6, "{sliders} sliders");
        // Les curseurs partent des couleurs du thème actuel (#0a0a0a → clarté 4 %).
        let st = seed(&widgets);
        assert_eq!(st["bl"], PanelValue::Int(4));
        assert_eq!(st["bh"], PanelValue::Int(0));

        // Bouger le curseur de clarté donne un aperçu en direct, de couleurs valides et lisibles.
        let mut light = st.clone();
        light.insert("bl".into(), PanelValue::Int(92));
        let effects = change(
            &plugin,
            themed_input(),
            "apply",
            PanelValue::Int(92),
            &light,
        );
        let [E::ThemePreview {
            dark,
            bg,
            fg,
            accent,
        }] = effects.as_slice()
        else {
            panic!("{effects:?}")
        };
        assert!(!dark);
        for c in [bg, fg, accent] {
            assert!(crate::theme::parse_hex(c).is_some(), "{c}");
        }
        let contrast = crate::theme::contrast(
            crate::theme::parse_hex(fg).unwrap(),
            crate::theme::parse_hex(bg).unwrap(),
        );
        assert!(
            contrast >= 4.5,
            "auto text must be readable, got {contrast}"
        );

        // Choisir un thème de départ recopie ses couleurs dans les curseurs.
        let effects = change(&plugin, themed_input(), "on_base", text("Dracula"), &st);
        assert!(effects
            .iter()
            .any(|e| matches!(e, E::PanelSet { id, .. } if id == "bl")));
        assert!(effects.contains(&E::ThemePreview {
            dark: true,
            bg: "#282a36".into(),
            fg: "#f8f8f2".into(),
            accent: "#bd93f9".into()
        }));
    }

    #[test]
    fn theme_studio_harmonies_import_random_and_save() {
        let plugin = find("theme-studio");
        let st = state(&[("bh", PanelValue::Int(100))]);
        for (name, expected) in [
            ("Complementary", 280),
            ("Analogous", 130),
            ("Triadic", 220),
            ("Split complementary", 250),
        ] {
            let effects = change(&plugin, themed_input(), "on_harmony", text(name), &st);
            assert!(
                effects.contains(&E::PanelSet {
                    id: "ch".into(),
                    value: PanelValue::Int(expected)
                }),
                "{name}: {effects:?}"
            );
        }
        assert!(change(&plugin, themed_input(), "on_harmony", text("Manual"), &st).is_empty());

        let effects = change(&plugin, themed_input(), "on_hex_bg", text("#336699"), &st);
        assert!(effects
            .iter()
            .any(|e| matches!(e, E::ThemePreview { bg, .. } if bg == "#336699")));
        assert!(change(&plugin, themed_input(), "on_hex_bg", text("zzz"), &st).is_empty());
        assert!(change(
            &plugin,
            themed_input(),
            "on_hex_accent",
            text("#ff8800"),
            &st
        )
        .iter()
        .any(|e| matches!(e, E::ThemePreview { accent, .. } if accent == "#ff8800")));

        for _ in 0..20 {
            let effects = button(&plugin, themed_input(), "randomize", "", &BTreeMap::new());
            let preview = effects.iter().find_map(|e| match e {
                E::ThemePreview { dark, bg, fg, .. } => Some((*dark, bg.clone(), fg.clone())),
                _ => None,
            });
            let (dark, bg, fg) = preview.expect("randomize must preview");
            let (bg, fg) = (
                crate::theme::parse_hex(&bg).unwrap(),
                crate::theme::parse_hex(&fg).unwrap(),
            );
            assert_eq!(dark, bg.luminance() < 0.3);
            assert!(crate::theme::contrast(fg, bg) >= 4.5);
        }

        let flipped = button(&plugin, themed_input(), "flip", "", &st);
        assert!(
            flipped
                .iter()
                .any(|e| matches!(e, E::ThemePreview { dark: false, .. })),
            "{flipped:?}"
        );

        let named = state(&[("name", text("Aurore"))]);
        let saved = button(&plugin, themed_input(), "save", "", &named);
        assert!(saved
            .iter()
            .any(|e| matches!(e, E::ThemeSave { name, .. } if name == "Aurore")));
        let default_name = button(&plugin, themed_input(), "save", "", &BTreeMap::new());
        assert!(default_name
            .iter()
            .any(|e| matches!(e, E::ThemeSave { name, .. } if name == "My theme")));
        assert!(button(&plugin, themed_input(), "revert", "", &st).contains(&E::ThemePreviewClear));
    }

    #[test]
    fn note_statistics_reports_totals_longest_notes_and_words() {
        let plugin = find("note-statistics");
        let mut input = sample_input();
        input.notes[0].body = "alpha alpha alpha bravo bravo charlie".into();
        let lines = texts(&render(&plugin, input, "render_stats", &BTreeMap::new())).join("\n");
        assert!(lines.contains("3 notes, 8 words"), "{lines}");
        assert!(lines.contains("1. First — 6 words"), "{lines}");
        assert!(lines.contains("alpha: 3"), "{lines}");
        assert!(lines.contains("bravo: 2"), "{lines}");
    }

    #[test]
    fn find_and_replace_counts_and_needs_confirmation() {
        let plugin = find("find-replace");
        let base = [
            ("find", text("two")),
            ("replace", text("2")),
            ("scope", text("All notes")),
            ("titles", PanelValue::Bool(false)),
        ];
        let counted = button(&plugin, sample_input(), "count", "", &state(&base));
        assert!(counted.contains(&E::PanelSet {
            id: "report".into(),
            value: text("1 match(es) in 1 note(s)")
        }));

        // Sans confirmation : rien n'est modifié.
        let refused = button(
            &plugin,
            sample_input(),
            "replace_now",
            "",
            &state(&[
                base[0].clone(),
                base[1].clone(),
                base[2].clone(),
                base[3].clone(),
                ("confirm", PanelValue::Bool(false)),
            ]),
        );
        assert!(
            refused.iter().all(|e| matches!(e, E::Status(_))),
            "{refused:?}"
        );

        let mut confirmed = base.to_vec();
        confirmed.push(("confirm", PanelValue::Bool(true)));
        let done = button(
            &plugin,
            sample_input(),
            "replace_now",
            "",
            &state(&confirmed),
        );
        assert!(
            done.contains(&E::UpdateNote {
                uid: "a".into(),
                title: "First".into(),
                body: "one 2 three".into()
            }),
            "{done:?}"
        );
        assert_eq!(
            done.iter()
                .filter(|e| matches!(e, E::UpdateNote { .. }))
                .count(),
            1
        );
        assert!(done.contains(&E::PanelSet {
            id: "confirm".into(),
            value: PanelValue::Bool(false)
        }));

        // Dans l'éditeur : « alpha » devient « omega ».
        let mut in_editor = vec![
            ("find", text("alpha")),
            ("replace", text("omega")),
            ("scope", text("Note being edited")),
            ("titles", PanelValue::Bool(false)),
            ("confirm", PanelValue::Bool(true)),
        ];
        let editor = button(
            &plugin,
            sample_input(),
            "replace_now",
            "",
            &state(&in_editor),
        );
        assert!(
            editor.contains(&E::EditorSetText("omega beta gamma delta".into())),
            "{editor:?}"
        );
        in_editor[0] = ("find", text(""));
        assert!(button(
            &plugin,
            sample_input(),
            "replace_now",
            "",
            &state(&in_editor)
        )
        .iter()
        .all(|e| matches!(e, E::Status(_))));
    }

    #[test]
    fn snippets_are_stored_listed_inserted_and_deleted() {
        let plugin = find("snippets");
        let saved = button(
            &plugin,
            sample_input(),
            "add",
            "",
            &state(&[("name", text("sig")), ("text", text("Cordialement"))]),
        );
        assert!(
            saved.contains(&E::StorageSet {
                key: "snippets".into(),
                value: "sig\tCordialement\n".into()
            }),
            "{saved:?}"
        );
        assert!(button(
            &plugin,
            sample_input(),
            "add",
            "",
            &state(&[("name", text("")), ("text", text("x"))])
        )
        .iter()
        .all(|e| matches!(e, E::Status(_))));

        let mut input = sample_input();
        input
            .storage
            .insert("snippets".into(), "sig\tHello⏎World\nbye\tSee you\n".into());
        let widgets = render(&plugin, input.clone(), "render_snippets", &BTreeMap::new());
        let buttons: Vec<_> = widgets
            .iter()
            .filter_map(|w| {
                if let Widget::Button { label, .. } = w {
                    Some(label.clone())
                } else {
                    None
                }
            })
            .collect();
        assert!(
            buttons.contains(&"Insert sig".to_string())
                && buttons.contains(&"Delete bye".to_string()),
            "{buttons:?}"
        );
        assert_eq!(
            button(&plugin, input.clone(), "insert", "sig", &BTreeMap::new()).first(),
            Some(&E::EditorInsert("Hello\nWorld".into()))
        );
        let removed = button(&plugin, input, "remove", "sig", &BTreeMap::new());
        assert!(removed.contains(&E::StorageSet {
            key: "snippets".into(),
            value: "bye\tSee you\n".into()
        }));
    }

    #[test]
    fn markdown_toolbar_fits_the_button_limit_and_inserts_markdown() {
        let plugin = find("markdown-toolbar");
        let effects = host::run_start(&plugin, &granted(&plugin), sample_input()).unwrap();
        let callbacks: Vec<String> = effects
            .iter()
            .filter_map(|e| {
                if let E::Button { callback, .. } = e {
                    Some(callback.clone())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(
            callbacks.len(),
            8,
            "the application shows at most 8 buttons per plugin"
        );
        for callback in callbacks {
            let out = call(&plugin, sample_input(), &callback, vec![]);
            assert!(
                matches!(out.as_slice(), [E::EditorInsert(t)] if !t.is_empty()),
                "{callback}"
            );
        }
    }

    #[test]
    fn theme_scheduler_night_hours_wrap_around_midnight() {
        let plugin = find("theme-scheduler");
        let night = |hour: i64, start: i64, end: i64| -> bool {
            host::run(
                &plugin,
                &granted(&plugin),
                themed_input(),
                "is_night",
                vec![hour.into(), start.into(), end.into()],
            )
            .unwrap()
            .value
            .unwrap()
            .as_bool()
            .unwrap()
        };
        assert!(night(23, 19, 7) && night(3, 19, 7) && night(19, 19, 7));
        assert!(!night(7, 19, 7) && !night(12, 19, 7) && !night(18, 19, 7));
        assert!(night(9, 8, 10) && !night(10, 8, 10) && !night(7, 8, 10));
        assert!(!night(5, 5, 5), "an empty range never switches");

        let id = |name: &str| {
            host::run(
                &plugin,
                &granted(&plugin),
                themed_input(),
                "id_of",
                vec![name.to_string().into()],
            )
            .unwrap()
            .value
            .unwrap()
            .into_string()
            .unwrap()
        };
        assert_eq!(id("Dracula"), "dracula");
        assert_eq!(id("Inconnu"), "default-dark");

        let saved = change(
            &plugin,
            themed_input(),
            "save_day",
            text("Dracula"),
            &BTreeMap::new(),
        );
        assert_eq!(
            saved,
            vec![E::StorageSet {
                key: "day_theme".into(),
                value: "dracula".into()
            }]
        );
    }

    #[test]
    fn daily_goal_shows_a_progress_bar() {
        let plugin = find("word-goal");
        let mut input = sample_input();
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        input.notes[0].updated_at = format!("{today}T10:00:00+00:00");
        input.notes[0].body = "word ".repeat(150);
        input.storage.insert("goal".into(), "300".into());
        let widgets = render(&plugin, input, "render_goal", &BTreeMap::new());
        let percent = widgets.iter().find_map(|w| {
            if let Widget::Progress { percent, .. } = w {
                Some(*percent)
            } else {
                None
            }
        });
        assert_eq!(percent, Some(50));
        assert_eq!(
            change(
                &plugin,
                sample_input(),
                "save_goal",
                PanelValue::Int(500),
                &BTreeMap::new()
            ),
            vec![E::StorageSet {
                key: "goal".into(),
                value: "500".into()
            }]
        );
    }
}
