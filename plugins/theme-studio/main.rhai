// Theme Studio — crée ton thème avec aperçu en direct.
// L'état du panneau (valeurs des curseurs…) est géré par l'application et passé à chaque fonction.

fn on_start() {
    ui_panel("Theme Studio", "render_studio");
}

fn val(state, key, fallback) {
    if key in state { state[key] } else { fallback }
}

// Valeurs de départ : les couleurs du thème actuellement appliqué.
fn defaults() {
    let colors = theme_colors(theme_current());
    if colors == () {
        colors = #{ bg: "#0a0a0a", fg: "#ffffff", accent: "#d95229", dark: true };
    }
    let b = color_to_hsl(colors.bg);
    let a = color_to_hsl(colors.accent);
    #{ bh: b.h, bs: b.s, bl: b.l, ch: a.h, cs: a.s, cl: a.l, tl: 90, auto_text: true }
}

fn get(state, key) {
    let d = defaults();
    if key in state { state[key] } else { d[key] }
}

// Calcule les trois couleurs à partir des curseurs.
fn palette(state) {
    let bg = color_hsl(get(state, "bh"), get(state, "bs"), get(state, "bl"));
    let accent = color_hsl(get(state, "ch"), get(state, "cs"), get(state, "cl"));
    let dark = get(state, "bl") < 55;
    let fg = "";
    if get(state, "auto_text") {
        let lightness = if dark { 94 } else { 8 };
        fg = color_hsl(get(state, "bh"), 15, lightness);
    } else {
        fg = color_hsl(get(state, "bh"), 15, get(state, "tl"));
    }
    #{ bg: bg, fg: fg, accent: accent, dark: dark }
}

fn preview_state(state) {
    let p = palette(state);
    theme_preview(p.bg, p.fg, p.accent, p.dark);
}

fn render_studio(state) {
    let p = palette(state);
    let ui = [];
    ui.push(#{ kind: "heading", text: "Theme Studio" });
    ui.push(#{ kind: "text", text: "Move a slider: the whole app changes live. Save when you like the result." });

    let names = ["(current theme)"];
    for t in theme_names() { names.push(t.name); }
    ui.push(#{ kind: "choice", id: "base", label: "Start from", options: names,
               value: "(current theme)", on_change: "on_base" });

    ui.push(#{ kind: "separator" });
    ui.push(#{ kind: "heading", text: "Background" });
    ui.push(#{ kind: "slider", id: "bh", label: "Hue", min: 0, max: 360, value: get(state, "bh"), on_change: "apply" });
    ui.push(#{ kind: "slider", id: "bs", label: "Saturation", min: 0, max: 100, value: get(state, "bs"), on_change: "apply" });
    ui.push(#{ kind: "slider", id: "bl", label: "Lightness", min: 0, max: 100, value: get(state, "bl"), on_change: "apply" });

    ui.push(#{ kind: "heading", text: "Accent" });
    ui.push(#{ kind: "choice", id: "harmony", label: "Harmony with the background",
               options: ["Manual", "Complementary", "Analogous", "Triadic", "Split complementary"],
               value: "Manual", on_change: "on_harmony" });
    ui.push(#{ kind: "slider", id: "ch", label: "Hue", min: 0, max: 360, value: get(state, "ch"), on_change: "apply" });
    ui.push(#{ kind: "slider", id: "cs", label: "Saturation", min: 0, max: 100, value: get(state, "cs"), on_change: "apply" });
    ui.push(#{ kind: "slider", id: "cl", label: "Lightness", min: 0, max: 100, value: get(state, "cl"), on_change: "apply" });

    ui.push(#{ kind: "heading", text: "Text" });
    ui.push(#{ kind: "toggle", id: "auto_text", label: "Pick a readable text color automatically",
               value: get(state, "auto_text"), on_change: "apply" });
    if !get(state, "auto_text") {
        ui.push(#{ kind: "slider", id: "tl", label: "Text lightness", min: 0, max: 100, value: get(state, "tl"), on_change: "apply" });
    }

    ui.push(#{ kind: "separator" });
    ui.push(#{ kind: "preview", bg: p.bg, fg: p.fg, accent: p.accent });
    ui.push(#{ kind: "swatches", colors: [p.bg, p.fg, p.accent] });
    ui.push(#{ kind: "text", text: `Background ${p.bg}    Text ${p.fg}    Accent ${p.accent}` });

    let text_ratio = color_contrast(p.fg, p.bg);
    let accent_ratio = color_contrast(p.accent, p.bg);
    let text_grade = "too low";
    if text_ratio >= 7.0 { text_grade = "AAA" } else if text_ratio >= 4.5 { text_grade = "AA" }
    let accent_grade = "low";
    if accent_ratio >= 3.0 { accent_grade = "ok" }
    ui.push(#{ kind: "text", text: `Text contrast ${text_ratio}:1 (${text_grade})    Accent contrast ${accent_ratio}:1 (${accent_grade}). Low contrasts are corrected when the theme is saved.` });

    ui.push(#{ kind: "separator" });
    ui.push(#{ kind: "heading", text: "Import colors" });
    ui.push(#{ kind: "input", id: "hex_bg", label: "Background hex (#rrggbb)", value: "", on_change: "on_hex_bg" });
    ui.push(#{ kind: "input", id: "hex_ac", label: "Accent hex (#rrggbb)", value: "", on_change: "on_hex_accent" });

    ui.push(#{ kind: "separator" });
    ui.push(#{ kind: "button", label: "Randomize", callback: "randomize", arg: "" });
    ui.push(#{ kind: "button", label: "Flip light / dark", callback: "flip", arg: "" });
    ui.push(#{ kind: "button", label: "Reset to the current theme", callback: "reset_all", arg: "" });
    ui.push(#{ kind: "button", label: "Revert the preview", callback: "revert", arg: "" });

    ui.push(#{ kind: "separator" });
    ui.push(#{ kind: "heading", text: "Save" });
    ui.push(#{ kind: "input", id: "name", label: "Theme name", value: "My theme" });
    ui.push(#{ kind: "button", label: "Save as a theme and apply", callback: "save", arg: "" });
    ui
}

// Un curseur ou une case a changé : l'état contient déjà la nouvelle valeur.
fn apply(value, state) {
    preview_state(state);
}

fn set_all(h) {
    panel_set("bh", h.bh); panel_set("bs", h.bs); panel_set("bl", h.bl);
    panel_set("ch", h.ch); panel_set("cs", h.cs); panel_set("cl", h.cl);
}

fn on_base(value, state) {
    if value == "(current theme)" {
        reset_all("", state);
        return;
    }
    for t in theme_names() {
        if t.name == value {
            let b = color_to_hsl(t.bg);
            let a = color_to_hsl(t.accent);
            set_all(#{ bh: b.h, bs: b.s, bl: b.l, ch: a.h, cs: a.s, cl: a.l });
            panel_set("auto_text", true);
            theme_preview(t.bg, t.fg, t.accent, t.dark);
            return;
        }
    }
}

fn on_harmony(value, state) {
    let offset = 0;
    if value == "Complementary" { offset = 180 }
    else if value == "Analogous" { offset = 30 }
    else if value == "Triadic" { offset = 120 }
    else if value == "Split complementary" { offset = 150 }
    else { return; }
    let hue = (get(state, "bh") + offset) % 360;
    panel_set("ch", hue);
    let next = state;
    next.ch = hue;
    preview_state(next);
}

fn on_hex_bg(value, state) {
    let c = color_to_hsl(value);
    if c == () { return; }
    panel_set("bh", c.h); panel_set("bs", c.s); panel_set("bl", c.l);
    let next = state;
    next.bh = c.h; next.bs = c.s; next.bl = c.l;
    preview_state(next);
}

fn on_hex_accent(value, state) {
    let c = color_to_hsl(value);
    if c == () { return; }
    panel_set("ch", c.h); panel_set("cs", c.s); panel_set("cl", c.l);
    let next = state;
    next.ch = c.h; next.cs = c.s; next.cl = c.l;
    preview_state(next);
}

fn randomize(arg, state) {
    let dark = random_int(0, 1) == 1;
    let h = random_int(0, 359);
    let next = #{
        bh: h,
        bs: random_int(10, 55),
        bl: if dark { random_int(6, 18) } else { random_int(90, 97) },
        ch: (h + random_int(0, 3) * 60 + 30) % 360,
        cs: random_int(55, 95),
        cl: if dark { random_int(58, 72) } else { random_int(32, 46) },
        auto_text: true
    };
    set_all(next);
    panel_set("auto_text", true);
    preview_state(next);
}

fn flip(arg, state) {
    let light = if get(state, "bl") < 55 { 94 } else { 8 };
    let next = state;
    next.bl = light;
    if light > 50 { next.cl = 40 } else { next.cl = 62 }
    panel_set("bl", next.bl);
    panel_set("cl", next.cl);
    preview_state(next);
}

fn reset_all(arg, state) {
    let d = defaults();
    set_all(d);
    panel_set("auto_text", true);
    theme_preview_clear();
}

fn revert(arg, state) {
    theme_preview_clear();
    ui_status("Preview reverted");
}

fn save(arg, state) {
    let p = palette(state);
    let name = val(state, "name", "My theme");
    theme_save(name, p.dark, p.bg, p.fg, p.accent);
    ui_status("Theme saved: " + name);
}
