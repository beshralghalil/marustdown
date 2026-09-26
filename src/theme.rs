use std::collections::BTreeMap;

use crate::config::{ColorValue, Config, Glyphs, Keys, Layout, StyleSpec, Styles};

pub type Color = u8; // index into Theme::palette; 0 = terminal default

pub const BOLD: u8 = 1 << 0;
pub const DIM: u8 = 1 << 1;
pub const ITALIC: u8 = 1 << 2;
pub const UNDERLINE: u8 = 1 << 3;
pub const STRIKE: u8 = 1 << 4;
pub const REVERSE: u8 = 1 << 5;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Style {
    pub fg: Color,
    pub bg: Color,
    pub attrs: u8,
    pub link: u16, // index into Document::links; 0 = none
}

impl Style {
    /// `top` layered over `self`: its colors and link win when set, attributes add up.
    pub fn over(self, top: Style) -> Style {
        Style {
            fg: if top.fg != 0 { top.fg } else { self.fg },
            bg: if top.bg != 0 { top.bg } else { self.bg },
            attrs: self.attrs | top.attrs,
            link: if top.link != 0 { top.link } else { self.link },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Paint {
    Rgb(u8, u8, u8),
    Indexed(u8),
}

pub struct Theme {
    palette: Vec<Paint>,
    pub styles: Styles<Style>,
    pub glyphs: Glyphs,
    pub layout: Layout,
    pub keys: Keys,
}

impl Theme {
    pub fn new(cfg: Config) -> Result<Theme, String> {
        validate(&cfg)?;
        let mut palette = Palette::new(&cfg.colors)?;
        let styles = cfg.styles.try_map(|name, spec| {
            palette
                .style(&spec)
                .map_err(|e| format!("styles.{name}: {e}"))
        })?;
        let mut paints = palette.paints;
        if !cfg.layout.color {
            paints.clear();
        }
        Ok(Theme {
            palette: paints,
            styles,
            glyphs: cfg.glyphs,
            layout: cfg.layout,
            keys: cfg.keys,
        })
    }

    pub fn paint(&self, c: Color) -> Option<Paint> {
        (c as usize)
            .checked_sub(1)
            .and_then(|i| self.palette.get(i))
            .copied()
    }
}

impl Styles<Style> {
    pub fn heading(&self, level: usize) -> Style {
        [self.h1, self.h2, self.h3, self.h4, self.h5, self.h6][level.clamp(1, 6) - 1]
    }
}

fn validate(cfg: &Config) -> Result<(), String> {
    let l = &cfg.layout;
    if !(1..=16).contains(&l.tab_width) {
        return Err("layout.tab_width: must be between 1 and 16".into());
    }
    if l.width == 0 {
        return Err("layout.width: must be at least 1".into());
    }
    if cfg.glyphs.bullets.is_empty() {
        return Err("glyphs.bullets: needs at least one entry".into());
    }
    Ok(())
}

struct Palette<'a> {
    paints: Vec<Paint>,
    named: BTreeMap<&'a str, Color>,
}

impl<'a> Palette<'a> {
    fn new(colors: &'a BTreeMap<String, ColorValue>) -> Result<Self, String> {
        let mut p = Palette {
            paints: Vec::new(),
            named: BTreeMap::new(),
        };
        for (name, value) in colors {
            let color = match value {
                ColorValue::Name(s) if s == "default" => 0,
                ColorValue::Name(s) if !s.starts_with('#') => {
                    return Err(format!(
                        "colors.{name}: expected \"#rrggbb\", \"default\" or 0-255"
                    ));
                }
                v => p.literal(v).map_err(|e| format!("colors.{name}: {e}"))?,
            };
            p.named.insert(name, color);
        }
        Ok(p)
    }

    fn style(&mut self, spec: &StyleSpec) -> Result<Style, String> {
        let flags = [
            (spec.bold, BOLD),
            (spec.dim, DIM),
            (spec.italic, ITALIC),
            (spec.underline, UNDERLINE),
            (spec.strike, STRIKE),
            (spec.reverse, REVERSE),
        ];
        Ok(Style {
            fg: spec.fg.as_ref().map_or(Ok(0), |v| self.color(v))?,
            bg: spec.bg.as_ref().map_or(Ok(0), |v| self.color(v))?,
            attrs: flags
                .iter()
                .filter(|(on, _)| *on)
                .fold(0, |a, (_, bit)| a | bit),
            link: 0,
        })
    }

    fn color(&mut self, value: &ColorValue) -> Result<Color, String> {
        match value {
            ColorValue::Name(s) if s == "default" => Ok(0),
            ColorValue::Name(s) if !s.starts_with('#') => self
                .named
                .get(s.as_str())
                .copied()
                .ok_or_else(|| format!("unknown color {s:?}")),
            v => self.literal(v),
        }
    }

    fn literal(&mut self, value: &ColorValue) -> Result<Color, String> {
        let paint = match value {
            ColorValue::Index(i) => Paint::Indexed(*i),
            ColorValue::Name(s) => parse_hex(s).ok_or_else(|| format!("invalid color {s:?}"))?,
        };
        if self.paints.len() >= Color::MAX as usize {
            return Err("too many distinct colors".into());
        }
        self.paints.push(paint);
        Ok(self.paints.len() as Color)
    }
}

fn parse_hex(s: &str) -> Option<Paint> {
    let hex = s.strip_prefix('#')?;
    let digit = |i: usize| u8::from_str_radix(hex.get(i..i + 1)?, 16).ok();
    match hex.len() {
        3 => Some(Paint::Rgb(digit(0)? * 17, digit(1)? * 17, digit(2)? * 17)),
        6 => {
            let byte = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
            Some(Paint::Rgb(byte(0)?, byte(2)?, byte(4)?))
        }
        _ => None,
    }
}

#[cfg(test)]
pub fn test_theme() -> Theme {
    Theme::new(crate::config::defaults()).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attribute_bits_are_distinct() {
        let all = [BOLD, DIM, ITALIC, UNDERLINE, STRIKE, REVERSE];
        assert!(all.iter().all(|c| c.count_ones() == 1));
        assert_eq!(
            all.iter().fold(0, |a, c| a | c).count_ones(),
            all.len() as u32
        );
        let s = BOLD | ITALIC;
        assert!(s & BOLD != 0 && s & ITALIC != 0 && s & DIM == 0);
    }

    #[test]
    fn default_style_is_empty() {
        let s = Style::default();
        assert_eq!((s.fg, s.bg, s.attrs, s.link), (0, 0, 0, 0));
        assert_eq!(std::mem::size_of::<Style>(), 6);
    }

    #[test]
    fn over_layers() {
        let base = Style {
            fg: 1,
            bg: 2,
            attrs: BOLD,
            link: 0,
        };
        let top = Style {
            fg: 3,
            bg: 0,
            attrs: ITALIC,
            link: 4,
        };
        assert_eq!(
            base.over(top),
            Style {
                fg: 3,
                bg: 2,
                attrs: BOLD | ITALIC,
                link: 4
            }
        );
    }

    #[test]
    fn resolves_palette() {
        let t = test_theme();
        assert_eq!(t.paint(0), None);
        assert_eq!(t.paint(t.styles.h1.fg), Some(Paint::Indexed(4)));
        assert_eq!(parse_hex("#fff"), Some(Paint::Rgb(255, 255, 255)));
        assert_eq!(parse_hex("#12345"), None);
    }

    #[test]
    fn no_color_drops_paints() {
        let mut cfg = crate::config::defaults();
        cfg.layout.color = false;
        let t = Theme::new(cfg).unwrap();
        assert_eq!(t.paint(t.styles.h1.fg), None);
        assert!(t.styles.h1.attrs & BOLD != 0);
    }
}
