fn on_start() {
    ui_button("notes", "Remove empty", "clean");
}

fn clean() {
    let removed = 0;
    for note in notes_list() {
        let title = note.title;
        let body = note.body;
        title.trim();
        body.trim();
        if title == "" && body == "" {
            notes_delete(note.id);
            removed += 1;
        }
    }
    ui_status(`${removed} empty note(s) removed`);
}
