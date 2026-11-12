fn on_start() {
    ui_panel("Snippets", "render_snippets");
}

// Format de stockage : une ligne par extrait, « nom<TAB>texte », les retours à la ligne du texte devenant « ⏎ ».
fn load() {
    let raw = storage_get("snippets");
    let items = [];
    if raw == () { return items; }
    for line in raw.split('\n') {
        let at = line.index_of("\t", 0);
        if at > 0 {
            items.push(#{ name: line.sub_string(0, at), text: line.sub_string(at + 1) });
        }
    }
    items
}

fn store(items) {
    let raw = "";
    for item in items { raw += item.name + "\t" + item.text + "\n"; }
    storage_set("snippets", raw);
}

fn render_snippets(state) {
    let items = load();
    let ui = [];
    ui.push(#{ kind: "heading", text: "Snippets" });
    if items.len() == 0 { ui.push(#{ kind: "text", text: "No snippet yet. Add one below." }); }
    for item in items {
        let preview = item.text;
        if preview.len() > 80 { preview = preview.sub_string(0, 80) + "…" }
        ui.push(#{ kind: "text", text: item.name + " — " + preview });
        ui.push(#{ kind: "button", label: "Insert " + item.name, callback: "insert", arg: item.name });
        ui.push(#{ kind: "button", label: "Delete " + item.name, callback: "remove", arg: item.name });
    }
    ui.push(#{ kind: "separator" });
    ui.push(#{ kind: "heading", text: "New snippet" });
    ui.push(#{ kind: "input", id: "name", label: "Name", value: "" });
    ui.push(#{ kind: "input", id: "text", label: "Text", value: "" });
    ui.push(#{ kind: "button", label: "Save snippet", callback: "add", arg: "" });
    ui
}

fn add(arg, state) {
    let name = state.name;
    name.trim();
    if name == "" || state.text == "" {
        ui_status("A snippet needs a name and a text");
        return;
    }
    name.replace("\t", " ");
    let text = state.text;
    text.replace("\t", " ");
    let items = [];
    for item in load() { if item.name != name { items.push(item); } }
    if items.len() >= 50 {
        ui_status("At most 50 snippets");
        return;
    }
    items.push(#{ name: name, text: text });
    store(items);
    panel_set("name", "");
    panel_set("text", "");
    ui_status("Snippet saved: " + name);
}

fn insert(arg, state) {
    for item in load() {
        if item.name == arg {
            let text = item.text;
            text.replace("⏎", "\n");
            editor_insert(text);
            ui_status("Inserted: " + arg);
            return;
        }
    }
}

fn remove(arg, state) {
    let items = [];
    for item in load() { if item.name != arg { items.push(item); } }
    store(items);
    ui_status("Deleted: " + arg);
}
