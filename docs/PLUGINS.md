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
