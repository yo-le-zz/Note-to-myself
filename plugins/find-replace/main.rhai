fn on_start() {
    ui_panel("Find and replace", "render_replace");
}

fn count_in(text, needle) {
    if needle == "" { return 0; }
    let total = 0;
    let at = text.index_of(needle, 0);
    while at >= 0 {
        total += 1;
        at = text.index_of(needle, at + needle.len());
    }
    total
}

fn render_replace(state) {
    let report = if "report" in state { state.report } else { "" };
    let ui = [];
    ui.push(#{ kind: "heading", text: "Find and replace" });
    ui.push(#{ kind: "input", id: "find", label: "Find", value: "" });
    ui.push(#{ kind: "input", id: "replace", label: "Replace with", value: "" });
    ui.push(#{ kind: "choice", id: "scope", label: "Where", options: ["All notes", "Note being edited"], value: "All notes" });
    ui.push(#{ kind: "toggle", id: "titles", label: "Also in titles", value: false });
    ui.push(#{ kind: "button", label: "Count matches", callback: "count", arg: "" });
    if report != "" { ui.push(#{ kind: "text", text: report }); }
    ui.push(#{ kind: "separator" });
    ui.push(#{ kind: "toggle", id: "confirm", label: "I understand that Replace changes my notes", value: false });
    ui.push(#{ kind: "button", label: "Replace", callback: "replace_now", arg: "" });
    ui
}

fn count(arg, state) {
    let needle = state.find;
    let total = 0;
    let touched = 0;
    if state.scope == "All notes" {
        for note in notes_list() {
            let c = count_in(note.body, needle);
            if state.titles { c += count_in(note.title, needle); }
            if c > 0 { touched += 1; total += c; }
        }
        panel_set("report", `${total} match(es) in ${touched} note(s)`);
    } else {
        total = count_in(editor_text(), needle);
        panel_set("report", `${total} match(es) in the note being edited`);
    }
}

fn replace_now(arg, state) {
    if !state.confirm {
        ui_status("Tick the confirmation first");
        return;
    }
    let needle = state.find;
    if needle == "" {
        ui_status("Nothing to find");
        return;
    }
    if state.scope == "All notes" {
        let changed = 0;
        for note in notes_list() {
            let body = note.body;
            let title = note.title;
            body.replace(needle, state.replace);
            if state.titles { title.replace(needle, state.replace); }
            if body != note.body || title != note.title {
                if changed < 200 {
                    notes_update(note.id, title, body);
                    changed += 1;
                }
            }
        }
        ui_status(`${changed} note(s) updated (at most 200 at a time)`);
    } else {
        let text = editor_text();
        text.replace(needle, state.replace);
        editor_set_text(text);
        ui_status("Replaced in the note being edited");
    }
    panel_set("confirm", false);
}
