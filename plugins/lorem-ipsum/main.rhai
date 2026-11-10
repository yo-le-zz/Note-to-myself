fn on_start() {
    ui_button("write", "Lorem", "insert_lorem");
}

fn insert_lorem() {
    editor_insert("Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.\n");
}
