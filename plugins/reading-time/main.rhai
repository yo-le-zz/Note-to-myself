fn on_start() {
    ui_button("write", "Reading time", "estimate");
}

fn estimate() {
    let words = word_count(editor_text());
    // Environ 200 mots par minute.
    let minutes = (words + 199) / 200;
    ui_status(`${words} words, about ${minutes} min`);
}
