fn on_start() {
    ui_note_action("Duplicate", "duplicate");
}

fn duplicate(note) {
    notes_add(note.title + " (copy)", note.body);
    ui_status("Note duplicated");
}
