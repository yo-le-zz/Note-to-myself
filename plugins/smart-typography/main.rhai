fn on_note_save(note) {
    let body = note.body;
    body.replace("...", "…");
    body.replace(" -- ", " — ");
    note.body = body;
    note
}
