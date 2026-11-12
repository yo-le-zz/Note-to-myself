fn on_start() {
    ui_panel("Daily word goal", "render_goal");
}

fn goal() {
    let g = storage_get("goal");
    if g == () { 300 } else { g.parse_int() }
}

fn words_today() {
    let today = now_date();
    let total = 0;
    for note in notes_list() {
        if note.updated.starts_with(today) { total += word_count(note.body); }
    }
    total
}

fn render_goal(state) {
    let target = if "goal" in state { state.goal } else { goal() };
    let done = words_today();
    let percent = if target > 0 { done * 100 / target } else { 0 };
    let ui = [];
    ui.push(#{ kind: "heading", text: "Daily word goal" });
    ui.push(#{ kind: "slider", id: "goal", label: "Words per day", min: 50, max: 5000, value: target, on_change: "save_goal" });
    ui.push(#{ kind: "progress", label: `${done} / ${target} words today`, percent: percent });
    if done >= target {
        ui.push(#{ kind: "text", text: "Goal reached. Well done!" });
    } else {
        ui.push(#{ kind: "text", text: `${target - done} words to go.` });
    }
    ui.push(#{ kind: "text", text: "Counts the words of every note modified today." });
    ui
}

fn save_goal(value, state) {
    storage_set("goal", value.to_string());
}
