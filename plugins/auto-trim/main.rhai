fn right_trim(text) {
    while text.len() > 0 && (text.ends_with(" ") || text.ends_with("\t") || text.ends_with("\r")) {
        text.pop();
    }
    text
}

fn on_note_save(note) {
    let result = "";
    let blank_run = 0;
    for line in note.body.split('\n') {
        let clean = right_trim(line);
        if clean == "" {
            blank_run += 1;
            // Au plus deux lignes vides consécutives.
            if blank_run > 2 { continue; }
        } else {
            blank_run = 0;
        }
        if result != "" || clean != "" {
            result += clean + "\n";
        }
    }
    while result.ends_with("\n") {
        result.pop();
    }
    note.body = result;
    note
}
