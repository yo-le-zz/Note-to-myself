fn on_note_save(note) {
    let title = note.title;
    title.trim();
    if title != "" {
        return note;
    }
    for line in note.body.split('\n') {
        let candidate = line;
        candidate.trim();
        if candidate != "" {
            if candidate.len() > 60 {
                candidate = candidate.sub_string(0, 60);
            }
            note.title = candidate;
            break;
        }
    }
    note
}
