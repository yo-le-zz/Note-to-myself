fn on_start() {
    ui_button("write", "Password", "insert_password");
}

fn insert_password() {
    editor_insert(random_password(20));
}
