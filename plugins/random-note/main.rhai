fn on_start() {
    ui_button("notes", "Random", "pick");
}

fn pick() {
    let count = notes_count();
    if count == 0 {
        ui_status("No notes yet");
        return;
    }
    let notes = notes_list();
    let note = notes[random_int(0, count - 1)];
    let title = note.title;
    if title == "" { title = "(untitled)"; }
    ui_status("Random note: " + title);
}
