fn on_start() {
    ui_button("write", "Checklist", "add_item");
}

fn add_item() {
    editor_insert("- [ ] ");
}
