fn on_start() {
    ui_panel("Theme scheduler", "render_schedule");
    if storage_get("enabled") == "true" {
        apply_now();
    }
}

fn setting(key, fallback) {
    let v = storage_get(key);
    if v == () { fallback } else { v }
}

// Vrai si `hour` est dans la plage de nuit [start, end[, qui peut passer minuit.
fn is_night(hour, start, end) {
    if start == end { return false; }
    if start < end { hour >= start && hour < end } else { hour >= start || hour < end }
}

fn current_hour() {
    now_iso().sub_string(11, 2).parse_int()
}

fn apply_now() {
    let start = setting("night_start", "19").parse_int();
    let end = setting("night_end", "7").parse_int();
    let id = if is_night(current_hour(), start, end) {
        setting("night_theme", "default-dark")
    } else {
        setting("day_theme", "default-light")
    };
    if id != theme_current() { theme_set(id); }
}

fn name_of(id) {
    for t in theme_names() { if t.id == id { return t.name; } }
    id
}

fn id_of(name) {
    for t in theme_names() { if t.name == name { return t.id; } }
    "default-dark"
}

fn render_schedule(state) {
    let names = [];
    for t in theme_names() { names.push(t.name); }
    let ui = [];
    ui.push(#{ kind: "heading", text: "Theme scheduler" });
    ui.push(#{ kind: "toggle", id: "enabled", label: "Switch theme automatically (checked at startup and with the button below)",
               value: setting("enabled", "false") == "true", on_change: "save_enabled" });
    ui.push(#{ kind: "choice", id: "day", label: "Day theme", options: names,
               value: name_of(setting("day_theme", "default-light")), on_change: "save_day" });
    ui.push(#{ kind: "choice", id: "night", label: "Night theme", options: names,
               value: name_of(setting("night_theme", "default-dark")), on_change: "save_night" });
    ui.push(#{ kind: "slider", id: "night_start", label: "Night starts at (hour)", min: 0, max: 23,
               value: setting("night_start", "19").parse_int(), on_change: "save_start" });
    ui.push(#{ kind: "slider", id: "night_end", label: "Night ends at (hour)", min: 0, max: 23,
               value: setting("night_end", "7").parse_int(), on_change: "save_end" });
    ui.push(#{ kind: "button", label: "Apply now", callback: "apply_button", arg: "" });
    ui
}

fn save_enabled(value, state) { storage_set("enabled", if value { "true" } else { "false" }); }
fn save_day(value, state) { storage_set("day_theme", id_of(value)); }
fn save_night(value, state) { storage_set("night_theme", id_of(value)); }
fn save_start(value, state) { storage_set("night_start", value.to_string()); }
fn save_end(value, state) { storage_set("night_end", value.to_string()); }

fn apply_button(arg, state) {
    apply_now();
    ui_status("Theme updated for the current time");
}
