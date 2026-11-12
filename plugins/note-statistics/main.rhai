fn on_start() {
    ui_panel("Note statistics", "render_stats");
}

fn is_stop(word) {
    let stops = ["the", "and", "that", "this", "with", "for", "are", "was", "but", "not", "you", "your",
                 "les", "des", "une", "est", "que", "qui", "pour", "dans", "pas", "sur", "avec", "mais", "vous"];
    stops.contains(word)
}

fn clean(word) {
    let w = word.to_lower();
    w.trim();
    w.replace(".", ""); w.replace(",", ""); w.replace(";", ""); w.replace(":", "");
    w.replace("!", ""); w.replace("?", ""); w.replace("\"", ""); w.replace("(", ""); w.replace(")", "");
    w
}

fn render_stats(state) {
    let scope = if "scope" in state { state.scope } else { "Last 50 notes" };
    let depth = 50;
    if scope == "Last 200 notes" { depth = 200 }
    else if scope == "Last 20 notes" { depth = 20 }

    let notes = notes_list();
    let total = notes.len();
    let words = 0;
    let chars = 0;
    let empty = 0;
    let longest = [];
    let counts = #{};

    for i in 0..total {
        let note = notes[i];
        let n = word_count(note.body);
        words += n;
        chars += note.body.len();
        if n == 0 && note.title == "" { empty += 1; }
        if i < depth {
            longest.push(#{ title: note.title, words: n });
            for raw in note.body.split(' ') {
                let w = clean(raw);
                if w.len() > 3 && !is_stop(w) {
                    if w in counts { counts[w] += 1 } else { counts[w] = 1 }
                }
            }
        }
    }

    let ui = [];
    ui.push(#{ kind: "heading", text: "Note statistics" });
    ui.push(#{ kind: "choice", id: "scope", label: "Detailed analysis of",
               options: ["Last 20 notes", "Last 50 notes", "Last 200 notes"], value: scope });
    ui.push(#{ kind: "separator" });
    let average = if total > 0 { words / total } else { 0 };
    ui.push(#{ kind: "text", text: `${total} notes, ${words} words, ${chars} characters` });
    ui.push(#{ kind: "text", text: `About ${average} words per note, ${empty} empty note(s)` });
    ui.push(#{ kind: "text", text: `Reading time of everything: about ${(words + 199) / 200} min` });

    longest.sort(|a, b| b.words - a.words);
    ui.push(#{ kind: "heading", text: "Longest notes" });
    let shown = if longest.len() < 5 { longest.len() } else { 5 };
    for i in 0..shown {
        let title = longest[i].title;
        if title == "" { title = "(untitled)" }
        ui.push(#{ kind: "text", text: `${i + 1}. ${title} — ${longest[i].words} words` });
    }

    ui.push(#{ kind: "heading", text: "Most used words" });
    let pairs = [];
    for w in counts.keys() { pairs.push(#{ word: w, count: counts[w] }); }
    pairs.sort(|a, b| b.count - a.count);
    let top = if pairs.len() < 10 { pairs.len() } else { 10 };
    if top == 0 { ui.push(#{ kind: "text", text: "Not enough text yet." }); }
    for i in 0..top {
        ui.push(#{ kind: "text", text: `${pairs[i].word}: ${pairs[i].count}` });
    }
    ui
}
