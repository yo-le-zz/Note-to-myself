//! Thèmes : ~80 palettes sélectionnées, ~100 générées (12 teintes × 8 styles), thèmes
//! personnalisés (`themes/*.json`) et surcharges (accent, arrondi, bordures, espacement…).
//!
//! Chaque thème ne définit que trois couleurs (fond, texte, accent) ; le reste est dérivé.
//! Le contraste est garanti par `readable` : une couleur trop proche du fond est corrigée.

use crate::settings::ThemeSettings;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }

    pub fn rgb_f32(self) -> [f32; 3] {
        [
            self.0 as f32 / 255.0,
            self.1 as f32 / 255.0,
            self.2 as f32 / 255.0,
        ]
    }

    /// Luminance relative (WCAG).
    pub fn luminance(self) -> f32 {
        let lin = |c: u8| {
            let c = c as f32 / 255.0;
            if c <= 0.03928 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * lin(self.0) + 0.7152 * lin(self.1) + 0.0722 * lin(self.2)
    }

    /// Mélange linéaire : `t = 0` donne `self`, `t = 1` donne `other`.
    pub fn mix(self, other: Rgb, t: f32) -> Rgb {
        let m = |a: u8, b: u8| {
            (a as f32 + (b as f32 - a as f32) * t)
                .round()
                .clamp(0.0, 255.0) as u8
        };
        Rgb(m(self.0, other.0), m(self.1, other.1), m(self.2, other.2))
    }
}

pub fn contrast(a: Rgb, b: Rgb) -> f32 {
    let (la, lb) = (a.luminance(), b.luminance());
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// Accepte `#rrggbb` ou `rrggbb`.
pub fn parse_hex(text: &str) -> Option<Rgb> {
    let hex = text.trim().trim_start_matches('#');
    if hex.len() != 6 || !hex.is_ascii() {
        return None;
    }
    let value = u32::from_str_radix(hex, 16).ok()?;
    Some(Rgb((value >> 16) as u8, (value >> 8) as u8, value as u8))
}

fn from_u32(value: u32) -> Rgb {
    Rgb((value >> 16) as u8, (value >> 8) as u8, value as u8)
}

/// Teinte (0 à 360), saturation et clarté (0 à 1) vers RGB.
pub fn hsl(h: f32, s: f32, l: f32) -> Rgb {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match (h % 360.0) as u32 / 60 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let to = |v: f32| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    Rgb(to(r), to(g), to(b))
}

/// RGB vers (teinte 0 à 360, saturation 0 à 1, clarté 0 à 1).
pub fn rgb_to_hsl(rgb: Rgb) -> (f32, f32, f32) {
    let [r, g, b] = rgb.rgb_f32();
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let l = (max + min) / 2.0;
    let d = max - min;
    if d < 1e-6 {
        return (0.0, 0.0, l);
    }
    let s = d / (1.0 - (2.0 * l - 1.0).abs());
    let h = if max == r {
        ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    ((h * 60.0).rem_euclid(360.0), s.clamp(0.0, 1.0), l)
}

/// Éloigne progressivement `color` du fond jusqu'à atteindre le contraste demandé.
pub fn readable(color: Rgb, bg: Rgb, min_ratio: f32) -> Rgb {
    let target = if bg.luminance() > 0.4 {
        Rgb(0, 0, 0)
    } else {
        Rgb(255, 255, 255)
    };
    let mut current = color;
    for step in 1..=20 {
        if contrast(current, bg) >= min_ratio {
            return current;
        }
        current = color.mix(target, step as f32 / 20.0);
    }
    current
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeDef {
    pub id: String,
    pub name: String,
    pub dark: bool,
    pub bg: Rgb,
    pub fg: Rgb,
    pub accent: Rgb,
}

impl ThemeDef {
    fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        dark: bool,
        bg: Rgb,
        fg: Rgb,
        accent: Rgb,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            dark,
            bg,
            fg: readable(fg, bg, 4.5),
            accent: readable(accent, bg, 3.0),
        }
    }
}

/// (identifiant, nom, sombre, fond, texte, accent)
type Curated = (&'static str, &'static str, bool, u32, u32, u32);

const CURATED: &[Curated] = &[
    (
        "default-dark",
        "Note Dark",
        true,
        0x0a0a0a,
        0xffffff,
        0xd95229,
    ),
    (
        "default-light",
        "Note Light",
        false,
        0xffffff,
        0x0a0a0a,
        0xd95229,
    ),
    ("dracula", "Dracula", true, 0x282a36, 0xf8f8f2, 0xbd93f9),
    (
        "dracula-pink",
        "Dracula Pink",
        true,
        0x282a36,
        0xf8f8f2,
        0xff79c6,
    ),
    ("nord", "Nord", true, 0x2e3440, 0xd8dee9, 0x88c0d0),
    (
        "nord-frost",
        "Nord Frost",
        true,
        0x242933,
        0xe5e9f0,
        0x81a1c1,
    ),
    (
        "solarized-dark",
        "Solarized Dark",
        true,
        0x002b36,
        0x93a1a1,
        0xb58900,
    ),
    (
        "solarized-light",
        "Solarized Light",
        false,
        0xfdf6e3,
        0x586e75,
        0xcb4b16,
    ),
    (
        "gruvbox-dark",
        "Gruvbox Dark",
        true,
        0x282828,
        0xebdbb2,
        0xfabd2f,
    ),
    (
        "gruvbox-light",
        "Gruvbox Light",
        false,
        0xfbf1c7,
        0x3c3836,
        0xb57614,
    ),
    (
        "gruvbox-material",
        "Gruvbox Material",
        true,
        0x1d2021,
        0xd4be98,
        0xa9b665,
    ),
    ("monokai", "Monokai", true, 0x272822, 0xf8f8f2, 0xa6e22e),
    (
        "monokai-pro",
        "Monokai Pro",
        true,
        0x2d2a2e,
        0xfcfcfa,
        0xffd866,
    ),
    (
        "tokyo-night",
        "Tokyo Night",
        true,
        0x1a1b26,
        0xc0caf5,
        0x7aa2f7,
    ),
    (
        "tokyo-storm",
        "Tokyo Storm",
        true,
        0x24283b,
        0xc0caf5,
        0xbb9af7,
    ),
    (
        "tokyo-day",
        "Tokyo Day",
        false,
        0xd5d6db,
        0x343b58,
        0x34548a,
    ),
    (
        "catppuccin-mocha",
        "Catppuccin Mocha",
        true,
        0x1e1e2e,
        0xcdd6f4,
        0xcba6f7,
    ),
    (
        "catppuccin-macchiato",
        "Catppuccin Macchiato",
        true,
        0x24273a,
        0xcad3f5,
        0xc6a0f6,
    ),
    (
        "catppuccin-frappe",
        "Catppuccin Frappé",
        true,
        0x303446,
        0xc6d0f5,
        0xca9ee6,
    ),
    (
        "catppuccin-latte",
        "Catppuccin Latte",
        false,
        0xeff1f5,
        0x4c4f69,
        0x8839ef,
    ),
    ("rose-pine", "Rosé Pine", true, 0x191724, 0xe0def4, 0xebbcba),
    (
        "rose-pine-moon",
        "Rosé Pine Moon",
        true,
        0x232136,
        0xe0def4,
        0xc4a7e7,
    ),
    (
        "rose-pine-dawn",
        "Rosé Pine Dawn",
        false,
        0xfaf4ed,
        0x575279,
        0xd7827e,
    ),
    ("one-dark", "One Dark", true, 0x282c34, 0xabb2bf, 0x61afef),
    (
        "one-light",
        "One Light",
        false,
        0xfafafa,
        0x383a42,
        0x4078f2,
    ),
    (
        "github-dark",
        "GitHub Dark",
        true,
        0x0d1117,
        0xc9d1d9,
        0x58a6ff,
    ),
    (
        "github-dimmed",
        "GitHub Dimmed",
        true,
        0x22272e,
        0xadbac7,
        0x539bf5,
    ),
    (
        "github-light",
        "GitHub Light",
        false,
        0xffffff,
        0x24292f,
        0x0969da,
    ),
    (
        "material-dark",
        "Material Dark",
        true,
        0x263238,
        0xeeffff,
        0x80cbc4,
    ),
    (
        "material-ocean",
        "Material Ocean",
        true,
        0x0f111a,
        0x8f93a2,
        0x84ffff,
    ),
    (
        "material-palenight",
        "Material Palenight",
        true,
        0x292d3e,
        0xa6accd,
        0xc792ea,
    ),
    (
        "material-lighter",
        "Material Lighter",
        false,
        0xfafafa,
        0x546e7a,
        0x00acc1,
    ),
    ("ayu-dark", "Ayu Dark", true, 0x0b0e14, 0xbfbdb6, 0xe6b450),
    (
        "ayu-mirage",
        "Ayu Mirage",
        true,
        0x1f2430,
        0xcccac2,
        0xffcc66,
    ),
    (
        "ayu-light",
        "Ayu Light",
        false,
        0xfafafa,
        0x5c6166,
        0xff9940,
    ),
    (
        "everforest-dark",
        "Everforest Dark",
        true,
        0x2b3339,
        0xd3c6aa,
        0xa7c080,
    ),
    (
        "everforest-light",
        "Everforest Light",
        false,
        0xfdf6e3,
        0x5c6a72,
        0x8da101,
    ),
    ("kanagawa", "Kanagawa", true, 0x1f1f28, 0xdcd7ba, 0x957fb8),
    (
        "kanagawa-dragon",
        "Kanagawa Dragon",
        true,
        0x181616,
        0xc5c9c5,
        0x8ba4b0,
    ),
    ("nightfox", "Nightfox", true, 0x192330, 0xcdcecf, 0x719cd6),
    ("carbonfox", "Carbonfox", true, 0x161616, 0xf2f4f8, 0x78a9ff),
    (
        "oceanic-next",
        "Oceanic Next",
        true,
        0x1b2b34,
        0xc0c5ce,
        0x6699cc,
    ),
    ("synthwave", "Synthwave", true, 0x262335, 0xffffff, 0xff7edb),
    ("cobalt", "Cobalt", true, 0x193549, 0xffffff, 0xffc600),
    ("night-owl", "Night Owl", true, 0x011627, 0xd6deeb, 0x82aaff),
    (
        "light-owl",
        "Light Owl",
        false,
        0xfbfbfb,
        0x403f53,
        0x4876d6,
    ),
    (
        "shades-of-purple",
        "Shades of Purple",
        true,
        0x2d2b55,
        0xffffff,
        0xfad000,
    ),
    ("panda", "Panda", true, 0x292a2b, 0xe6e6e6, 0xff75b5),
    ("horizon", "Horizon", true, 0x1c1e26, 0xd5d8da, 0xe95678),
    ("moonlight", "Moonlight", true, 0x222436, 0xc8d3f5, 0x82aaff),
    ("zenburn", "Zenburn", true, 0x3f3f3f, 0xdcdccc, 0xf0dfaf),
    (
        "tomorrow-night",
        "Tomorrow Night",
        true,
        0x1d1f21,
        0xc5c8c6,
        0x81a2be,
    ),
    (
        "tomorrow-light",
        "Tomorrow Light",
        false,
        0xffffff,
        0x4d4d4c,
        0x4271ae,
    ),
    (
        "base16-ocean",
        "Base16 Ocean",
        true,
        0x2b303b,
        0xc0c5ce,
        0x8fa1b3,
    ),
    (
        "hacker-green",
        "Hacker Green",
        true,
        0x000000,
        0x33ff66,
        0x33ff66,
    ),
    (
        "amber-terminal",
        "Amber Terminal",
        true,
        0x0d0800,
        0xffb000,
        0xffb000,
    ),
    ("cyberpunk", "Cyberpunk", true, 0x0b0c10, 0xe0e0e0, 0xfcee0a),
    ("vaporwave", "Vaporwave", true, 0x1a1033, 0xf5e6ff, 0x01cdfe),
    ("sunset", "Sunset", true, 0x2b1b2e, 0xffe9d6, 0xff7e5f),
    ("aurora", "Aurora", true, 0x0f1b2d, 0xe8f1f2, 0x5ef2c0),
    ("forest", "Forest", true, 0x14201a, 0xdfeee3, 0x6fcf97),
    (
        "ocean-deep",
        "Deep Ocean",
        true,
        0x071a2b,
        0xd9ecf7,
        0x39a0ed,
    ),
    (
        "lavender-dark",
        "Lavender Night",
        true,
        0x1f1b2e,
        0xe8e0ff,
        0xb794f6,
    ),
    ("coffee", "Coffee", true, 0x1e1511, 0xecdccb, 0xd4a373),
    ("slate", "Slate", true, 0x1f2933, 0xe4e7eb, 0x63b3ed),
    ("ruby", "Ruby", true, 0x1a0b0e, 0xf8e1e4, 0xf0506e),
    ("emerald", "Emerald", true, 0x07140f, 0xd8f5e6, 0x2ecc71),
    ("sapphire", "Sapphire", true, 0x0a1226, 0xdbe4ff, 0x4c7dff),
    (
        "midnight-oled",
        "Midnight OLED",
        true,
        0x000000,
        0xe6e6e6,
        0x8ab4f8,
    ),
    (
        "high-contrast-dark",
        "High Contrast Dark",
        true,
        0x000000,
        0xffffff,
        0xffff00,
    ),
    (
        "high-contrast-light",
        "High Contrast Light",
        false,
        0xffffff,
        0x000000,
        0x0000cc,
    ),
    (
        "paper-white",
        "Paper White",
        false,
        0xfaf8f2,
        0x2b2b2b,
        0x9c4a1a,
    ),
    ("sepia", "Sepia", false, 0xf4ecd8, 0x5b4636, 0xa0522d),
    ("mint-light", "Mint", false, 0xe8f5ee, 0x1d3b2f, 0x1b8a5a),
    ("sky-light", "Sky", false, 0xe8f3fb, 0x14304a, 0x1f6fb2),
    ("blush-light", "Blush", false, 0xfbeef0, 0x4a2530, 0xc2385a),
    (
        "lavender-light",
        "Lavender Day",
        false,
        0xf1ecfb,
        0x2e2548,
        0x6d4bd0,
    ),
    ("peach-light", "Peach", false, 0xfdf0e6, 0x4a2f1d, 0xd2691e),
    ("sand-light", "Sand", false, 0xf3ead9, 0x3d3426, 0x8a6d3b),
];

const HUES: [(&str, f32); 12] = [
    ("Red", 2.0),
    ("Orange", 24.0),
    ("Amber", 42.0),
    ("Lime", 82.0),
    ("Green", 138.0),
    ("Teal", 172.0),
    ("Cyan", 192.0),
    ("Blue", 216.0),
    ("Indigo", 245.0),
    ("Violet", 272.0),
    ("Magenta", 305.0),
    ("Rose", 340.0),
];

/// (nom, sombre, saturation fond, clarté fond, saturation texte, clarté texte, saturation accent, clarté accent)
type StyleRow = (&'static str, bool, f32, f32, f32, f32, f32, f32);

const STYLES: [StyleRow; 8] = [
    ("Midnight", true, 0.45, 0.07, 0.20, 0.90, 0.85, 0.62),
    ("Dusk", true, 0.25, 0.15, 0.15, 0.88, 0.75, 0.66),
    ("Charcoal", true, 0.06, 0.11, 0.05, 0.90, 0.70, 0.60),
    ("Neon", true, 0.30, 0.05, 0.10, 0.93, 1.00, 0.60),
    ("Pastel", false, 0.60, 0.95, 0.30, 0.18, 0.55, 0.40),
    ("Paper", false, 0.25, 0.96, 0.15, 0.14, 0.60, 0.38),
    ("Mist", false, 0.20, 0.89, 0.20, 0.13, 0.55, 0.36),
    ("Ink", false, 0.00, 1.00, 0.30, 0.08, 0.90, 0.33),
];

fn generated() -> Vec<ThemeDef> {
    let mut themes = Vec::with_capacity(HUES.len() * STYLES.len());
    for (hue_name, hue) in HUES {
        for (style, dark, bs, bl, ts, tl, acs, acl) in STYLES {
            themes.push(ThemeDef::new(
                format!("{}-{}", hue_name.to_lowercase(), style.to_lowercase()),
                format!("{hue_name} {style}"),
                dark,
                hsl(hue, bs, bl),
                hsl(hue, ts, tl),
                hsl(hue, acs, acl),
            ));
        }
    }
    themes
}

/// Tous les thèmes intégrés, dans l'ordre d'affichage.
pub fn builtin_themes() -> Vec<ThemeDef> {
    let mut themes: Vec<ThemeDef> = CURATED
        .iter()
        .map(|&(id, name, dark, bg, fg, accent)| {
            ThemeDef::new(id, name, dark, from_u32(bg), from_u32(fg), from_u32(accent))
        })
        .collect();
    themes.extend(generated());
    themes
}

/// Construit un thème depuis des couleurs `#rrggbb` ; le texte et l'accent sont rendus lisibles.
pub fn theme_from_hex(
    name: &str,
    dark: bool,
    bg: &str,
    fg: &str,
    accent: &str,
) -> Option<ThemeDef> {
    Some(ThemeDef::new(
        String::new(),
        name.trim(),
        dark,
        parse_hex(bg)?,
        parse_hex(fg)?,
        parse_hex(accent)?,
    ))
}

/// Thème personnalisé tel qu'écrit dans `themes/*.json`.
#[derive(Debug, Serialize, Deserialize)]
struct UserThemeFile {
    name: String,
    dark: bool,
    bg: String,
    fg: String,
    accent: String,
}

fn slug(name: &str) -> String {
    let slug: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    slug.split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

pub fn user_theme_from_json(json: &str) -> Option<ThemeDef> {
    let file: UserThemeFile = serde_json::from_str(json).ok()?;
    let name = file.name.trim().chars().take(40).collect::<String>();
    let id = slug(&name);
    if id.is_empty() {
        return None;
    }
    Some(ThemeDef::new(
        format!("user-{id}"),
        name,
        file.dark,
        parse_hex(&file.bg)?,
        parse_hex(&file.fg)?,
        parse_hex(&file.accent)?,
    ))
}

pub fn user_theme_to_json(theme: &ThemeDef) -> String {
    let file = UserThemeFile {
        name: theme.name.clone(),
        dark: theme.dark,
        bg: theme.bg.to_hex(),
        fg: theme.fg.to_hex(),
        accent: theme.accent.to_hex(),
    };
    serde_json::to_string_pretty(&file).unwrap_or_default()
}

/// Charge les thèmes personnalisés ; les fichiers invalides, trop gros ou en double sont ignorés.
pub fn load_user_themes(dir: &Path) -> Vec<ThemeDef> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut themes: Vec<ThemeDef> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let small = entry.metadata().map(|m| m.len() < 4096).unwrap_or(false);
        let Some(theme) = small
            .then(|| std::fs::read_to_string(&path).ok())
            .flatten()
            .and_then(|t| user_theme_from_json(&t))
        else {
            continue;
        };
        if !themes.iter().any(|t| t.id == theme.id) {
            themes.push(theme);
        }
    }
    themes.sort_by(|a, b| a.name.cmp(&b.name));
    themes
}

pub fn save_user_theme(dir: &Path, theme: &ThemeDef) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let file = dir.join(format!("{}.json", theme.id.trim_start_matches("user-")));
    std::fs::write(file, user_theme_to_json(theme))
}

/// Palette finale utilisée par l'interface.
#[derive(Debug, Clone, PartialEq)]
pub struct Palette {
    pub dark: bool,
    pub bg: Rgb,
    pub panel: Rgb,
    pub fg: Rgb,
    pub muted: Rgb,
    pub accent: Rgb,
    pub radius: f32,
    pub border_width: f32,
    pub spacing: f32,
    pub editor_monospace: bool,
    pub ui_monospace: bool,
}

pub fn find<'a>(themes: &'a [ThemeDef], id: &str) -> Option<&'a ThemeDef> {
    themes.iter().find(|t| t.id == id)
}

/// Combine un thème de base et les surcharges de l'utilisateur.
pub fn resolve(base: &ThemeDef, settings: &ThemeSettings) -> Palette {
    let accent = settings
        .accent
        .as_deref()
        .and_then(parse_hex)
        .map(|a| readable(a, base.bg, 3.0))
        .unwrap_or(base.accent);
    // Le panneau se détache du fond vers le texte ; `panel_contrast` règle l'écart.
    let step = 0.06 + settings.panel_contrast as f32 * 0.008;
    let panel = base.bg.mix(base.fg, step.clamp(0.0, 0.2));
    Palette {
        dark: base.dark,
        bg: base.bg,
        panel,
        fg: base.fg,
        muted: readable(base.fg.mix(base.bg, 0.4), base.bg, 3.0),
        accent,
        radius: settings.radius as f32,
        border_width: settings.border_width as f32,
        spacing: settings.spacing as f32 / 100.0,
        editor_monospace: settings.editor_monospace,
        ui_monospace: settings.ui_monospace,
    }
}

/// Thème à utiliser selon les réglages ; retombe sur clair/sombre si l'identifiant est inconnu
/// (par exemple un thème personnalisé supprimé).
pub fn active_palette(themes: &[ThemeDef], settings: &ThemeSettings, legacy_dark: bool) -> Palette {
    let fallback = if legacy_dark {
        "default-dark"
    } else {
        "default-light"
    };
    let base = find(themes, &settings.id)
        .or_else(|| find(themes, fallback))
        .or_else(|| themes.first())
        .cloned()
        .unwrap_or_else(|| {
            ThemeDef::new(
                "fallback",
                "Fallback",
                true,
                Rgb(10, 10, 10),
                Rgb(255, 255, 255),
                Rgb(217, 82, 41),
            )
        });
    resolve(&base, settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn at_least_150_unique_themes() {
        let themes = builtin_themes();
        assert!(themes.len() >= 150, "only {} themes", themes.len());
        let ids: HashSet<_> = themes.iter().map(|t| &t.id).collect();
        let names: HashSet<_> = themes.iter().map(|t| &t.name).collect();
        assert_eq!(ids.len(), themes.len(), "duplicate theme id");
        assert_eq!(names.len(), themes.len(), "duplicate theme name");
        assert!(themes.iter().any(|t| t.dark) && themes.iter().any(|t| !t.dark));
    }

    #[test]
    fn every_theme_is_readable() {
        for theme in builtin_themes() {
            assert!(contrast(theme.fg, theme.bg) >= 4.5, "{} text", theme.id);
            assert!(
                contrast(theme.accent, theme.bg) >= 3.0,
                "{} accent",
                theme.id
            );
            let palette = resolve(&theme, &ThemeSettings::default());
            assert!(
                contrast(palette.muted, palette.bg) >= 3.0,
                "{} muted",
                theme.id
            );
            // Le texte reste lisible sur le panneau, même avec un contraste de panneau maximal.
            let extreme = resolve(
                &theme,
                &ThemeSettings {
                    panel_contrast: 10,
                    ..Default::default()
                },
            );
            assert!(
                contrast(extreme.fg, extreme.panel) >= 3.5,
                "{} panel",
                theme.id
            );
        }
    }

    #[test]
    fn dark_flag_matches_background() {
        for theme in builtin_themes() {
            assert_eq!(theme.dark, theme.bg.luminance() < 0.3, "{}", theme.id);
        }
    }

    #[test]
    fn hex_parsing() {
        assert_eq!(parse_hex("#ff8000"), Some(Rgb(255, 128, 0)));
        assert_eq!(parse_hex("00ff00"), Some(Rgb(0, 255, 0)));
        for bad in ["", "#fff", "#gggggg", "#12345678", "é12345"] {
            assert_eq!(parse_hex(bad), None, "{bad}");
        }
        assert_eq!(Rgb(1, 2, 255).to_hex(), "#0102ff");
    }

    #[test]
    fn readable_fixes_low_contrast_only() {
        let bg = Rgb(20, 20, 20);
        let dim = Rgb(30, 30, 30);
        assert!(contrast(readable(dim, bg, 3.0), bg) >= 3.0);
        let fine = Rgb(240, 240, 240);
        assert_eq!(readable(fine, bg, 3.0), fine);
    }

    #[test]
    fn accent_override_is_applied_and_made_readable() {
        let themes = builtin_themes();
        let base = find(&themes, "default-dark").unwrap();
        let settings = ThemeSettings {
            accent: Some("#202020".into()),
            ..Default::default()
        };
        let palette = resolve(base, &settings);
        assert_ne!(palette.accent, base.accent);
        assert!(contrast(palette.accent, base.bg) >= 3.0);
        let invalid = ThemeSettings {
            accent: Some("zzz".into()),
            ..Default::default()
        };
        assert_eq!(resolve(base, &invalid).accent, base.accent);
    }

    #[test]
    fn customization_metrics_flow_into_the_palette() {
        let themes = builtin_themes();
        let settings = ThemeSettings {
            radius: 20,
            border_width: 3,
            spacing: 120,
            editor_monospace: true,
            ..Default::default()
        };
        let palette = active_palette(&themes, &settings, true);
        assert_eq!(
            (palette.radius, palette.border_width, palette.spacing),
            (20.0, 3.0, 1.2)
        );
        assert!(palette.editor_monospace && !palette.ui_monospace);
    }

    #[test]
    fn unknown_theme_falls_back_to_legacy_mode() {
        let themes = builtin_themes();
        let settings = ThemeSettings {
            id: "supprime".into(),
            ..Default::default()
        };
        assert!(active_palette(&themes, &settings, true).dark);
        assert!(!active_palette(&themes, &settings, false).dark);
        assert!(active_palette(&[], &settings, true).dark);
    }

    #[test]
    fn user_themes_roundtrip_and_validation() {
        let json = r##"{"name":"Mon thème","dark":true,"bg":"#101820","fg":"#f0f0f0","accent":"#ff5500"}"##;
        let theme = user_theme_from_json(json).unwrap();
        assert_eq!(theme.id, "user-mon-th-me");
        let again = user_theme_from_json(&user_theme_to_json(&theme)).unwrap();
        assert_eq!(theme.bg, again.bg);
        for bad in [
            r##"{"name":"x","dark":true,"bg":"nope","fg":"#ffffff","accent":"#ffffff"}"##,
            r##"{"name":"   ","dark":true,"bg":"#000000","fg":"#ffffff","accent":"#ffffff"}"##,
            "pas du json",
        ] {
            assert!(user_theme_from_json(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn user_theme_files_are_loaded_from_disk() {
        let dir = std::env::temp_dir().join(format!("ntm-themes-{}", crate::store::new_uid()));
        let theme = user_theme_from_json(
            r##"{"name":"Disque","dark":false,"bg":"#ffffff","fg":"#111111","accent":"#0044cc"}"##,
        )
        .unwrap();
        save_user_theme(&dir, &theme).unwrap();
        std::fs::write(dir.join("casse.json"), "{").unwrap();
        std::fs::write(dir.join("ignore.txt"), "x").unwrap();
        let loaded = load_user_themes(&dir);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "Disque");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn hsl_roundtrip_is_stable() {
        for rgb in [
            Rgb(217, 82, 41),
            Rgb(0, 0, 0),
            Rgb(255, 255, 255),
            Rgb(30, 144, 255),
            Rgb(120, 120, 120),
            Rgb(10, 200, 90),
        ] {
            let (h, s, l) = rgb_to_hsl(rgb);
            let back = hsl(h, s, l);
            for (a, b) in [(rgb.0, back.0), (rgb.1, back.1), (rgb.2, back.2)] {
                assert!((a as i32 - b as i32).abs() <= 1, "{rgb:?} -> {back:?}");
            }
        }
        let (h, s, l) = rgb_to_hsl(Rgb(255, 0, 0));
        assert_eq!(
            (
                h.round() as i32,
                (s * 100.0).round() as i32,
                (l * 100.0).round() as i32
            ),
            (0, 100, 50)
        );
        // Un gris n'a pas de teinte ni de saturation.
        assert_eq!(rgb_to_hsl(Rgb(128, 128, 128)).1, 0.0);
    }
}
