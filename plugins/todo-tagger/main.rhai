fn on_note_save(note) {
    if note.body.contains("TODO") && !note.title.starts_with("[TODO]") {
        note.title = "[TODO] " + note.title;
    }
    note
}
