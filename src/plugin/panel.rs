//! Panneaux de plugin : une interface déclarative que le plugin décrit et que l'application dessine.
//!
//! Le plugin ne dessine rien lui-même : sa fonction de rendu renvoie une liste de composants (tableaux de
//! tables Rhai) que `parse_widgets` valide et borne. L'état des composants (valeurs des curseurs, champs…)
//! est conservé par l'application et transmis au plugin à chaque appel.

use rhai::{Dynamic, Map};

pub const MAX_WIDGETS: usize = 150;
const MAX_LABEL: usize = 100;
const MAX_TEXT: usize = 4000;
const MAX_OPTIONS: usize = 300;

/// Valeur d'un composant interactif.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PanelValue {
    Int(i64),
    Bool(bool),
    Text(String),
}

impl PanelValue {
    pub fn to_dynamic(&self) -> Dynamic {
        match self {
            Self::Int(i) => Dynamic::from(*i),
            Self::Bool(b) => Dynamic::from(*b),
            Self::Text(s) => Dynamic::from(s.clone()),
        }
    }

    /// Convertit une valeur de script ; les nombres à virgule sont arrondis.
    pub fn from_dynamic(value: &Dynamic) -> Option<PanelValue> {
        if let Ok(i) = value.as_int() {
            return Some(Self::Int(i));
        }
        if let Ok(f) = value.as_float() {
            return f.is_finite().then(|| Self::Int(f.round() as i64));
        }
        if let Ok(b) = value.as_bool() {
            return Some(Self::Bool(b));
        }
        value
            .clone()
            .into_string()
            .ok()
            .filter(|s| s.len() <= MAX_TEXT)
            .map(Self::Text)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Widget {
    Heading(String),
    Text(String),
    Separator,
    Slider {
        id: String,
        label: String,
        min: i64,
        max: i64,
        value: i64,
        on_change: Option<String>,
    },
    Input {
        id: String,
        label: String,
        value: String,
        on_change: Option<String>,
    },
    Toggle {
        id: String,
        label: String,
        value: bool,
        on_change: Option<String>,
    },
    Choice {
        id: String,
        label: String,
        options: Vec<String>,
        value: String,
        on_change: Option<String>,
    },
    Color {
        id: String,
        label: String,
        value: String,
        on_change: Option<String>,
    },
    Swatches(Vec<String>),
    Preview {
        bg: String,
        fg: String,
        accent: String,
    },
    Progress {
        label: String,
        percent: i64,
    },
    Button {
        label: String,
        callback: String,
        arg: String,
    },
}

impl Widget {
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::Slider { id, .. }
            | Self::Input { id, .. }
            | Self::Toggle { id, .. }
            | Self::Choice { id, .. }
            | Self::Color { id, .. } => Some(id),
            _ => None,
        }
    }

    pub fn on_change(&self) -> Option<&str> {
        match self {
            Self::Slider { on_change, .. }
            | Self::Input { on_change, .. }
            | Self::Toggle { on_change, .. }
            | Self::Choice { on_change, .. }
            | Self::Color { on_change, .. } => on_change.as_deref(),
            _ => None,
        }
    }

    /// Valeur initiale proposée par le plugin (utilisée tant que l'utilisateur n'a rien changé).
    pub fn default_value(&self) -> Option<PanelValue> {
        match self {
            Self::Slider { value, .. } => Some(PanelValue::Int(*value)),
            Self::Toggle { value, .. } => Some(PanelValue::Bool(*value)),
            Self::Input { value, .. } | Self::Choice { value, .. } | Self::Color { value, .. } => {
                Some(PanelValue::Text(value.clone()))
            }
            _ => None,
        }
    }
}

/// Nom de fonction ou d'identifiant : lettres, chiffres et `_`, sans commencer par un chiffre.
pub fn valid_name(name: &str, max: usize) -> bool {
    !name.is_empty()
        && name.len() <= max
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !name.starts_with(|c: char| c.is_ascii_digit())
}

fn text_field(map: &Map, key: &str, max: usize) -> Result<String, String> {
    match map.get(key) {
        None => Ok(String::new()),
        Some(v) => {
            let s = v
                .clone()
                .into_string()
                .map_err(|_| format!("`{key}` must be a string"))?;
            if s.chars().count() > max {
                return Err(format!("`{key}` is longer than {max} characters"));
            }
            Ok(s)
        }
    }
}

fn int_field(map: &Map, key: &str, default: i64) -> Result<i64, String> {
    match map.get(key) {
        None => Ok(default),
        Some(v) => PanelValue::from_dynamic(v)
            .and_then(|p| {
                if let PanelValue::Int(i) = p {
                    Some(i)
                } else {
                    None
                }
            })
            .ok_or_else(|| format!("`{key}` must be a number")),
    }
}

fn id_field(map: &Map) -> Result<String, String> {
    let id = text_field(map, "id", 32)?;
    if valid_name(&id, 32) {
        Ok(id)
    } else {
        Err("`id` must be 1-32 letters, digits or `_`".into())
    }
}

fn callback_field(map: &Map, key: &str) -> Result<Option<String>, String> {
    let name = text_field(map, key, 48)?;
    if name.is_empty() {
        Ok(None)
    } else if valid_name(&name, 48) {
        Ok(Some(name))
    } else {
        Err(format!("`{key}` must be a function name"))
    }
}

fn parse_widget(map: &Map) -> Result<Widget, String> {
    let kind = text_field(map, "kind", 20)?;
    let label = || text_field(map, "label", MAX_LABEL);
    Ok(match kind.as_str() {
        "heading" => Widget::Heading(text_field(map, "text", MAX_LABEL)?),
        "text" => Widget::Text(text_field(map, "text", MAX_TEXT)?),
        "separator" => Widget::Separator,
        "slider" => {
            let (min, max) = (int_field(map, "min", 0)?, int_field(map, "max", 100)?);
            if min >= max || !(-1_000_000_000..=1_000_000_000).contains(&min) || max > 1_000_000_000
            {
                return Err("slider needs min < max within ±1e9".into());
            }
            Widget::Slider {
                id: id_field(map)?,
                label: label()?,
                min,
                max,
                value: int_field(map, "value", min)?.clamp(min, max),
                on_change: callback_field(map, "on_change")?,
            }
        }
        "input" => Widget::Input {
            id: id_field(map)?,
            label: label()?,
            value: text_field(map, "value", 2000)?,
            on_change: callback_field(map, "on_change")?,
        },
        "toggle" => Widget::Toggle {
            id: id_field(map)?,
            label: label()?,
            value: map
                .get("value")
                .and_then(|v| v.as_bool().ok())
                .unwrap_or(false),
            on_change: callback_field(map, "on_change")?,
        },
        "choice" => {
            let options: Vec<String> = map
                .get("options")
                .and_then(|v| v.clone().into_typed_array::<String>().ok())
                .ok_or("`options` must be an array of strings")?;
            if options.is_empty()
                || options.len() > MAX_OPTIONS
                || options.iter().any(|o| o.chars().count() > 80)
            {
                return Err("choice needs 1-300 options of at most 80 characters".into());
            }
            let value = text_field(map, "value", 80)?;
            let value = if options.contains(&value) {
                value
            } else {
                options[0].clone()
            };
            Widget::Choice {
                id: id_field(map)?,
                label: label()?,
                options,
                value,
                on_change: callback_field(map, "on_change")?,
            }
        }
        "color" => Widget::Color {
            id: id_field(map)?,
            label: label()?,
            value: text_field(map, "value", 7)?,
            on_change: callback_field(map, "on_change")?,
        },
        "swatches" => {
            let colors: Vec<String> = map
                .get("colors")
                .and_then(|v| v.clone().into_typed_array::<String>().ok())
                .ok_or("`colors` must be an array of strings")?;
            if colors.len() > 40 {
                return Err("at most 40 swatches".into());
            }
            Widget::Swatches(colors)
        }
        "preview" => Widget::Preview {
            bg: text_field(map, "bg", 7)?,
            fg: text_field(map, "fg", 7)?,
            accent: text_field(map, "accent", 7)?,
        },
        "progress" => Widget::Progress {
            label: label()?,
            percent: int_field(map, "percent", 0)?.clamp(0, 100),
        },
        "button" => Widget::Button {
            label: label()?,
            callback: callback_field(map, "callback")?.ok_or("a button needs a `callback`")?,
            arg: text_field(map, "arg", 200)?,
        },
        other => return Err(format!("unknown widget kind `{other}`")),
    })
}

/// Valide la valeur renvoyée par la fonction de rendu d'un plugin.
pub fn parse_widgets(value: Dynamic) -> Result<Vec<Widget>, String> {
    let array = value
        .into_array()
        .map_err(|_| "the render function must return an array".to_string())?;
    if array.len() > MAX_WIDGETS {
        return Err(format!("a panel can hold at most {MAX_WIDGETS} widgets"));
    }
    let mut widgets = Vec::with_capacity(array.len());
    let mut ids = std::collections::HashSet::new();
    for (index, item) in array.into_iter().enumerate() {
        let map = item
            .try_cast::<Map>()
            .ok_or_else(|| format!("widget {index} is not a map"))?;
        let widget = parse_widget(&map).map_err(|e| format!("widget {index}: {e}"))?;
        if let Some(id) = widget.id() {
            if !ids.insert(id.to_string()) {
                return Err(format!("widget {index}: duplicate id `{id}`"));
            }
        }
        widgets.push(widget);
    }
    Ok(widgets)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(script: &str) -> Result<Vec<Widget>, String> {
        let engine = rhai::Engine::new();
        let value: Dynamic = engine.eval(script).map_err(|e| e.to_string())?;
        parse_widgets(value)
    }

    #[test]
    fn every_widget_kind_parses() {
        let widgets = parse(
            r##"[
                #{kind:"heading", text:"H"}, #{kind:"text", text:"T"}, #{kind:"separator"},
                #{kind:"slider", id:"s", label:"S", min:0, max:360, value:900, on_change:"cb"},
                #{kind:"input", id:"i", label:"I", value:"v"},
                #{kind:"toggle", id:"t", label:"T", value:true},
                #{kind:"choice", id:"c", label:"C", options:["a","b"], value:"zzz"},
                #{kind:"color", id:"k", label:"K", value:"#ff0000"},
                #{kind:"swatches", colors:["#000000","#ffffff"]},
                #{kind:"preview", bg:"#000000", fg:"#ffffff", accent:"#ff0000"},
                #{kind:"progress", label:"P", percent:150},
                #{kind:"button", label:"B", callback:"go_on", arg:"x"}
            ]"##,
        )
        .unwrap();
        assert_eq!(widgets.len(), 12);
        // Valeurs hors bornes ramenées dans les bornes ; choix inconnu → première option.
        assert!(
            matches!(&widgets[3], Widget::Slider { value: 360, on_change: Some(cb), .. } if cb == "cb")
        );
        assert!(matches!(&widgets[6], Widget::Choice { value, .. } if value == "a"));
        assert!(matches!(
            &widgets[10],
            Widget::Progress { percent: 100, .. }
        ));
        assert_eq!(widgets[3].default_value(), Some(PanelValue::Int(360)));
        assert_eq!(widgets[4].id(), Some("i"));
    }

    #[test]
    fn malformed_panels_are_rejected_with_a_reason() {
        for (script, expected) in [
            ("42", "array"),
            ("[1]", "not a map"),
            (r#"[#{kind:"nope"}]"#, "unknown widget"),
            (r#"[#{kind:"slider", id:"s", min:5, max:5}]"#, "min < max"),
            (r#"[#{kind:"slider", id:"bad id", min:0, max:5}]"#, "`id`"),
            (r#"[#{kind:"input", label:"no id"}]"#, "`id`"),
            (r#"[#{kind:"button", label:"b"}]"#, "callback"),
            (
                r#"[#{kind:"button", label:"b", callback:"not a name"}]"#,
                "function name",
            ),
            (r#"[#{kind:"choice", id:"c", options:[]}]"#, "options"),
            (r#"[#{kind:"choice", id:"c", options:[1,2]}]"#, "options"),
            (
                r#"[#{kind:"toggle", id:"a"}, #{kind:"toggle", id:"a"}]"#,
                "duplicate",
            ),
            (r#"[#{kind:"text", text: 5}]"#, "string"),
        ] {
            let error = parse(script).unwrap_err();
            assert!(error.contains(expected), "{script}: {error}");
        }
    }

    #[test]
    fn size_limits_hold() {
        let many = format!(
            "[{}]",
            vec![r#"#{kind:"separator"}"#; MAX_WIDGETS + 1].join(",")
        );
        assert!(parse(&many).unwrap_err().contains("at most"));
        let long = format!(r#"[#{{kind:"heading", text:"{}"}}]"#, "x".repeat(101));
        assert!(parse(&long).is_err());
    }

    #[test]
    fn panel_values_convert_both_ways() {
        let engine = rhai::Engine::new();
        let float: Dynamic = engine.eval("3.6").unwrap();
        assert_eq!(PanelValue::from_dynamic(&float), Some(PanelValue::Int(4)));
        let nan: Dynamic = engine.eval("0.0 / 0.0").unwrap();
        assert_eq!(PanelValue::from_dynamic(&nan), None);
        let unit: Dynamic = engine.eval("()").unwrap();
        assert_eq!(PanelValue::from_dynamic(&unit), None);
        for value in [
            PanelValue::Int(7),
            PanelValue::Bool(true),
            PanelValue::Text("é".into()),
        ] {
            assert_eq!(PanelValue::from_dynamic(&value.to_dynamic()), Some(value));
        }
        assert!(
            valid_name("ok_1", 10)
                && !valid_name("1no", 10)
                && !valid_name("a-b", 10)
                && !valid_name("", 10)
        );
    }
}
