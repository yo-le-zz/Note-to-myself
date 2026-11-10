fn on_start() {
    ui_button("home", "Daily note", "create_daily");
}

fn create_daily() {
    let title = "Daily " + now_date();
    for note in notes_list() {
        if note.title == title {
            ui_status("Today's note already exists");
            return;
        }
    }
    notes_add(title, "");
    ui_status("Created: " + title);
}
