// Plugin d'exemple : un bouton « Hello » dans l'éditeur.
// Les permissions (ui.button, editor.write, hook.start) sont déduites de ce code.
fn on_start() {
    ui_button("write", "Hello", "say_hello");
}

fn say_hello() {
    editor_insert("Hello from a plugin, " + now_date() + "\n");
}
