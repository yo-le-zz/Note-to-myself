fn on_start() {
    ui_button("write", "✅", "ok");
    ui_button("write", "⭐", "star");
    ui_button("write", "❗", "alert");
    ui_button("write", "💡", "idea");
}

fn ok() { editor_insert("✅ "); }
fn star() { editor_insert("⭐ "); }
fn alert() { editor_insert("❗ "); }
fn idea() { editor_insert("💡 "); }
