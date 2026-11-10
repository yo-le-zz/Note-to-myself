// Statistiques globales affichées dans la barre de statut.
fn on_start() {
    ui_button("notes", "Stats", "show_stats");
}

fn show_stats() {
    let words = 0;
    let chars = 0;
    for note in notes_list() {
        words += word_count(note.body);
        chars += note.body.len();
    }
    ui_status(`${notes_count()} notes, ${words} words, ${chars} characters`);
}
