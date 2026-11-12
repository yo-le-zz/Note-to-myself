# Writing plugins

A plugin is a folder with two files:

```
plugins/my-plugin/
├── plugin.toml
└── main.rhai
```

```toml
id = "my-plugin"          # a-z, 0-9 and '-', must equal the folder name
name = "My plugin"
version = "1.0.0"
author = "you"
description = "What it does"
# entry = "main.rhai"     # optional
```

Plugins are written in [Rhai](https://rhai.rs), a small scripting language (JavaScript/Rust-like).
**Do not declare permissions**: they are detected from your code and shown to the user, who must
approve them. Changing the code resets the approval.

Try the example: `examples/hello-plugin`, and check a plugin with
`cargo run --bin ntm-plugin-check -- plugins/my-plugin`.

## Entry points

Define functions with these names and the app calls them:

| Function | When | Permission |
| --- | --- | --- |
| `fn on_start()` | At startup and after unlocking. Register buttons here. | `hook.start` |
| `fn on_note_save(note)` | Before a note is saved. Return the (modified) `note` map with `title` and `body`. | `hook.note_save` |

Button and menu callbacks are plain functions you name yourself:
`ui_button("write", "Label", "my_callback")` calls `fn my_callback()`. Note menu actions receive the
note: `ui_note_action("Label", "cb")` calls `fn cb(note)`.

## API

| Function | Permission | Description |
| --- | --- | --- |
| `notes_list()` | `notes.read` | Array of notes, newest first. A note is a map: `id`, `title`, `body`, `created`, `updated` |
| `notes_get(id)` | `notes.read` | One note, or `()` |
| `notes_search(text)` | `notes.read` | Notes whose title or body contains `text` |
| `notes_count()` | `notes.read` | Number of notes |
| `notes_add(title, body)` | `notes.write` | Create a note |
| `notes_update(id, title, body)` | `notes.write` | Replace a note's content |
| `notes_delete(id)` | `notes.delete` | Delete a note |
| `editor_title()`, `editor_text()` | `editor.read` | Current editor content |
| `editor_set_title(s)`, `editor_set_text(s)`, `editor_insert(s)` | `editor.write` | Change the editor |
| `clipboard_set(s)` | `clipboard.write` | Copy text |
| `ui_button(zone, label, callback)` | `ui.button` | `zone` is `"home"`, `"notes"` or `"write"` |
| `ui_note_action(label, callback)` | `ui.note_action` | Entry in each note's menu |
| `ui_status(message)` | `ui.status` | Show a message |
| `theme_set(id)`, `theme_current()` | `theme.set`, `settings.read` | Theme ids are listed in the Themes screen |
| `setting_get(key)` | `settings.read` | `language`, `font_size`, `dark_mode`, `compact_mode` |
| `storage_get(key)`, `storage_set(key, value)` | `storage` | Private key/value store (encrypted when the vault is on) |
| `now_date()`, `now_iso()`, `random_int(a, b)`, `random_password(n)`, `word_count(s)` | none | Helpers |

Write functions (`notes_add`, `editor_insert`, …) record a request that the app applies after your
function returns; they do not return a value. The API deliberately has **no network, file or process
access**.

## Panels: a user interface of your own

A plugin can open a full panel with sliders, fields, toggles, choices, color swatches, a live preview,
progress bars and buttons. You do not draw anything: you **describe** the panel, the app draws it.

```rhai
fn on_start() {
    ui_panel("My panel", "render");      // adds an opener on the home screen (permission ui.panel)
}

fn render(state) {                       // `state` holds the current value of every widget by id
    [
        #{ kind: "heading", text: "Volume" },
        #{ kind: "slider", id: "vol", label: "Level", min: 0, max: 10, value: 5, on_change: "changed" },
        #{ kind: "button", label: "Reset", callback: "reset", arg: "" }
    ]
}

fn changed(value, state) { ui_status("Level " + value); }   // called when the widget changes
fn reset(arg, state) { panel_set("vol", 5); }                // buttons receive their `arg` and the state
```

Widgets (a map with a `kind`):

| kind | fields |
| --- | --- |
| `heading`, `text` | `text` |
| `separator` | |
| `slider` | `id`, `label`, `min`, `max`, `value` (integers), `on_change` |
| `input` | `id`, `label`, `value`, `on_change` |
| `toggle` | `id`, `label`, `value` (bool), `on_change` |
| `choice` | `id`, `label`, `options` (array of strings), `value`, `on_change` |
| `color` | `id`, `label`, `value` (`#rrggbb`), `on_change` |
| `swatches` | `colors` (array of `#rrggbb`) |
| `preview` | `bg`, `fg`, `accent`: a sample of the app drawn with these colors |
| `progress` | `label`, `percent` (0-100) |
| `button` | `label`, `callback`, `arg` |

The app owns the widget values. `value` is only the initial value; after that, the user's value is
used and handed back in `state` (`state.vol`, `"vol" in state`). Use `panel_set(id, value)` to change one
from a callback (it may also hold any other state key, for example a report text). After every change the
render function is called again, so the panel always reflects the state. Callbacks return nothing; they
ask for things through the API. At most 150 widgets, 2 panels per plugin, 64 state keys.

Colors and themes (the panel shows a live preview of the whole app):

| Function | Permission | Description |
| --- | --- | --- |
| `theme_preview(bg, fg, accent, dark)` | `theme.edit` | Re-colors the whole app live, until you leave the panel. Nothing is saved |
| `theme_preview_clear()` | `theme.edit` | Back to the real theme |
| `theme_save(name, dark, bg, fg, accent)` | `theme.edit` | Saves a custom theme and applies it (low contrast is corrected) |
| `theme_names()` | `settings.read` | Array of `#{id, name, dark, bg, fg, accent}` |
| `theme_colors(id)` | `settings.read` | One such map, or `()` |
| `color_hsl(h, s, l)` | none | `h` 0-360, `s` and `l` 0-100 → `"#rrggbb"` |
| `color_to_hsl(hex)` | none | `#{h, s, l}` or `()` |
| `color_mix(a, b, t)` | none | Mix two colors, `t` from 0 to 1 |
| `color_contrast(a, b)` | none | WCAG contrast ratio |
| `color_readable(color, bg, min)` | none | Adjusts `color` until it reaches the contrast `min` |

`plugins/theme-studio` is a complete example: HSL sliders, color harmonies, hex import, randomize,
light/dark flip, contrast report, live preview of the whole application, and one-click save.

Reserved words of Rhai cannot be used as function names (`go`, `is`, `in`, `do`, `fn`, `let`, `if`…).
Top-level statements of a script never run: only your functions are called.

## Rules

Rejected at load time: `eval`, `Fn`, `call`, `curry`, `is_def_fn`, `is_def_var`, `global`, `import`,
`export`, scripts over 128 KiB, and syntax errors. At run time: 500 000 operations and 2 seconds per
call, 256 requested actions per call.

## Submitting a plugin

1. Add your folder under `plugins/` (folder name = `id`).
2. Run `cargo run --bin ntm-plugin-check -- plugins/my-plugin`.
3. Open a pull request. The *Plugins* workflow re-runs the checker on every plugin folder.
   Maintainers review the code and the detected permissions; a plugin asking for more than it needs
   will be asked to change.

Built-in plugins (`plugins/`) are embedded at compile time by `src/plugin/builtin.rs` and are
exercised by tests against sample notes: add yours there and a test next to the others if you want it
shipped with the app.
