fn on_start() {
    ui_button("home", "Dark", "go_dark");
    ui_button("home", "Light", "go_light");
}

fn go_dark() { theme_set("default-dark"); }
fn go_light() { theme_set("default-light"); }
