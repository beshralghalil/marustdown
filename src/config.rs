use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::{env, fs};

use serde::Deserialize;
use serde::de::{self, Deserializer, Visitor};
use toml::{Table, Value};

const DEFAULT: &str = include_str!("../assets/config.toml");
const ASCII: &str = include_str!("../assets/ascii.toml");
const NO_COLOR: &str = include_str!("../assets/nocolor.toml");
const PRESETS: [(&str, &str); 3] = [
    ("dark", include_str!("../assets/themes/dark.toml")),
    ("light", include_str!("../assets/themes/light.toml")),
    ("ansi", include_str!("../assets/themes/ansi.toml")),
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub layout: Layout,
    pub colors: BTreeMap<String, ColorValue>,
    pub styles: Styles<StyleSpec>,
    pub glyphs: Glyphs,
    pub keys: Keys,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    pub width: usize,
    pub margin: usize,
    pub center: bool,
    pub line_numbers: bool,
    pub tab_width: usize,
    pub scroll_off: usize,
    pub scroll_duration: u64,
    pub color: bool,
    pub icons: bool,
    pub status_bar: bool,
    pub math: bool,
    pub diagrams: bool,
    pub watch: bool,
}

impl Layout {
    /// Content width and left margin for a terminal `cols` wide.
    pub fn fit(&self, cols: usize) -> (usize, usize) {
        let width = self.width.min(cols.saturating_sub(2 * self.margin)).max(1);
        let margin = if self.center {
            cols.saturating_sub(width) / 2
        } else {
            self.margin
        };
        (width, margin)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ColorValue {
    Index(u8),
    Name(String),
}

impl<'de> Deserialize<'de> for ColorValue {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl Visitor<'_> for V {
            type Value = ColorValue;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a color name, \"#rrggbb\", \"default\" or 0-255")
            }
            fn visit_i64<E: de::Error>(self, n: i64) -> Result<ColorValue, E> {
                u8::try_from(n)
                    .map(ColorValue::Index)
                    .map_err(|_| E::invalid_value(de::Unexpected::Signed(n), &self))
            }
            fn visit_str<E: de::Error>(self, s: &str) -> Result<ColorValue, E> {
                Ok(ColorValue::Name(s.to_owned()))
            }
        }
        d.deserialize_any(V)
    }
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct StyleSpec {
    pub fg: Option<ColorValue>,
    pub bg: Option<ColorValue>,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    pub reverse: bool,
}

/// Declares a struct with one field per name, generic over the field type,
/// so the same list serves config specs and resolved styles or key maps.
macro_rules! fields {
    ($name:ident { $($field:ident),* $(,)? }) => {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        pub struct $name<T> { $(pub $field: T,)* }

        #[allow(dead_code)]
        impl<T> $name<T> {
            pub fn entries(&self) -> impl Iterator<Item = (&'static str, &T)> {
                [$((stringify!($field), &self.$field),)*].into_iter()
            }

            pub fn try_map<U, E>(
                self,
                mut f: impl FnMut(&'static str, T) -> Result<U, E>,
            ) -> Result<$name<U>, E> {
                Ok($name { $($field: f(stringify!($field), self.$field)?,)* })
            }
        }
    };
}

fields!(Styles {
    text,
    h1,
    h2,
    h3,
    h4,
    h5,
    h6,
    strong,
    emphasis,
    strike,
    code,
    math,
    diagram,
    diagram_border,
    link,
    link_icon,
    image,
    quote,
    quote_bar,
    alert_note,
    alert_tip,
    alert_important,
    alert_warning,
    alert_caution,
    bullet,
    number,
    task_done,
    task_todo,
    rule,
    code_block,
    code_border,
    code_label,
    line_number,
    table_border,
    table_header,
    syntax_keyword,
    syntax_string,
    syntax_number,
    syntax_comment,
    syntax_type,
    syntax_function,
    syntax_constant,
    syntax_operator,
    syntax_tag,
    syntax_attribute,
    syntax_inserted,
    syntax_deleted,
    cursor,
    status,
    search,
    outline_level,
    hint,
    link_selected,
    overflow,
});

fields!(KeyTable {
    down,
    up,
    page_down,
    page_up,
    half_down,
    half_up,
    top,
    bottom,
    next_heading,
    prev_heading,
    search,
    next_match,
    prev_match,
    toggle,
    next_link,
    prev_link,
    open,
    hints,
    back,
    copy,
    outline,
    edit,
    scroll_left,
    scroll_right,
    scroll_home,
    scroll_end,
    quit,
});

pub type Keys = KeyTable<Vec<String>>;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Glyphs {
    pub heading: [String; 6],
    pub heading_rule: [String; 6],
    pub bullets: Vec<String>,
    pub task_done: String,
    pub task_todo: String,
    pub quote: String,
    pub link: String,
    pub image: String,
    pub rule: String,
    pub ellipsis: String,
    pub overflow_left: String,
    pub overflow_right: String,
    pub code_copy: String,
    pub code_box: [String; 6],
    pub table_box: [String; 11],
    pub alert_note: String,
    pub alert_tip: String,
    pub alert_important: String,
    pub alert_warning: String,
    pub alert_caution: String,
    pub breadcrumb: String,
    pub separator: String,
}

/// Command-line settings that decide which built-in layers apply.
#[derive(Default)]
pub struct Overrides<'a> {
    pub theme: Option<&'a str>,
    pub no_icons: bool,
    pub no_color: bool,
}

/// Loads `path`, or the user config at the XDG location when there is one,
/// layered over the built-in defaults.
pub fn load(path: Option<&Path>, overrides: &Overrides) -> Result<Config, String> {
    let dir = config_dir();
    let path = path.map(Path::to_path_buf).or_else(|| {
        dir.as_ref()
            .map(|d| d.join("config.toml"))
            .filter(|p| p.is_file())
    });
    let user = path.as_deref().map(read).transpose()?;
    build(user, dir.as_deref(), overrides).map_err(|e| match &path {
        Some(p) => format!("{}: {e}", p.display()),
        None => e,
    })
}

pub fn config_dir() -> Option<PathBuf> {
    let base = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("marustdown"))
}

fn build(user: Option<Table>, dir: Option<&Path>, overrides: &Overrides) -> Result<Config, String> {
    let setting = |section: Option<&str>, key: &str| {
        let table = user.as_ref()?;
        let table = match section {
            Some(s) => table.get(s)?.as_table()?,
            None => table,
        };
        table.get(key).cloned()
    };
    let enabled = |key| {
        setting(Some("layout"), key)
            .and_then(|v| v.as_bool())
            .unwrap_or(true)
    };
    let icons = !overrides.no_icons && enabled("icons");
    let color = !overrides.no_color && enabled("color");
    let mut merged = parse(DEFAULT);
    let theme = match (overrides.theme, setting(None, "theme")) {
        (Some(t), _) => t.to_owned(),
        (None, Some(Value::String(s))) => s,
        (None, Some(_)) => return Err("theme: expected a string".into()),
        (None, None) => merged["theme"].as_str().expect("built-in theme").to_owned(),
    };
    merge(&mut merged, preset(&theme, dir)?);
    if !icons {
        merge(&mut merged, parse(ASCII));
    }
    if !color {
        merge(&mut merged, parse(NO_COLOR));
    }
    if let Some(user) = user {
        merge(&mut merged, user);
    }
    merged.remove("theme");
    let mut cfg: Config = merged.try_into().map_err(|e| e.to_string())?;
    cfg.layout.icons = icons;
    cfg.layout.color = color;
    Ok(cfg)
}

fn preset(name: &str, dir: Option<&Path>) -> Result<Table, String> {
    if let Some((_, text)) = PRESETS.iter().find(|(n, _)| *n == name) {
        return Ok(parse(text));
    }
    match dir.map(|d| d.join("themes").join(format!("{name}.toml"))) {
        Some(path) if path.is_file() => read(&path),
        _ => Err(format!(
            "unknown theme {name:?}; built-in themes are dark, light and ansi"
        )),
    }
}

fn read(path: &Path) -> Result<Table, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    text.parse().map_err(|e| format!("{}: {e}", path.display()))
}

fn parse(text: &str) -> Table {
    text.parse().expect("built-in config is valid TOML")
}

/// Deep merge: tables merge recursively, every other value replaces.
fn merge(base: &mut Table, over: Table) {
    for (key, value) in over {
        match (base.get_mut(&key), value) {
            (Some(Value::Table(b)), Value::Table(o)) => merge(b, o),
            (_, value) => {
                base.insert(key, value);
            }
        }
    }
}

#[cfg(test)]
pub fn defaults() -> Config {
    build(None, None, &Overrides::default()).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(text: &str) -> Result<Config, String> {
        build(Some(text.parse().unwrap()), None, &Overrides::default())
    }

    #[test]
    fn defaults_parse() {
        let cfg = defaults();
        assert_eq!(cfg.layout.width, 90);
        assert_eq!(cfg.colors["accent"], ColorValue::Index(4));
    }

    #[test]
    fn user_values_merge_key_by_key() {
        let cfg = user("[styles.h1]\nfg = \"red\"\n[layout]\nwidth = 70").unwrap();
        assert_eq!(cfg.layout.width, 70);
        assert!(cfg.layout.center);
        assert_eq!(cfg.styles.h1.fg, Some(ColorValue::Name("red".into())));
        assert!(cfg.styles.h1.bold);
    }

    #[test]
    fn unknown_keys_are_errors() {
        assert!(user("[styles.h7]\nbold = true").is_err());
        assert!(user("[layout]\nwidht = 3").is_err());
    }

    #[test]
    fn presets_and_ascii() {
        let light = user("theme = \"light\"").unwrap();
        assert_eq!(light.colors["accent"], ColorValue::Name("#1e66f5".into()));
        assert!(user("theme = \"nope\"").is_err());
        let no_icons = Overrides {
            no_icons: true,
            ..Overrides::default()
        };
        let ascii = build(None, None, &no_icons).unwrap();
        assert_eq!(ascii.glyphs.task_done, "[x]");
        assert!(!ascii.layout.icons);
    }

    #[test]
    fn no_color_reverses_the_cursor() {
        assert!(!defaults().styles.cursor.reverse);
        let no_color = Overrides {
            no_color: true,
            ..Overrides::default()
        };
        let cfg = build(None, None, &no_color).unwrap();
        assert!(cfg.styles.cursor.reverse);
        assert!(!cfg.layout.color);
        let from_file = user("[layout]\ncolor = false").unwrap();
        assert!(from_file.styles.cursor.reverse);
        let overridden =
            user("[layout]\ncolor = false\n[styles.cursor]\nreverse = false\nbold = true").unwrap();
        assert!(!overridden.styles.cursor.reverse && overridden.styles.cursor.bold);
    }

    #[test]
    fn user_glyphs_win_over_ascii() {
        let cfg = user("[layout]\nicons = false\n[glyphs]\nrule = \"~\"").unwrap();
        assert_eq!(cfg.glyphs.rule, "~");
        assert_eq!(cfg.glyphs.task_todo, "[ ]");
    }

    #[test]
    fn fit_centers_and_clamps() {
        let l = defaults().layout;
        assert_eq!(l.fit(200), (90, 55));
        assert_eq!(l.fit(50), (46, 2));
    }
}
