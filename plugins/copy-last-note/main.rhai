fn on_start() {
    ui_button("home", "Copy last", "copy_last");
}

fn copy_last() {
    let notes = notes_list();
    if notes.len() == 0 {
        ui_status("No notes yet");
        return;
    }
    let note = notes[0];
    clipboard_set(note.title + "\n\n" + note.body);
    ui_status("Last note copied");
}
