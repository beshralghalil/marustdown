use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::config::{KeyTable, Keys};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Down,
    Up,
    PageDown,
    PageUp,
    HalfDown,
    HalfUp,
    Top,
    Bottom,
    NextHeading,
    PrevHeading,
    Search,
    NextMatch,
    PrevMatch,
    Toggle,
    NextLink,
    PrevLink,
    Open,
    Hints,
    Back,
    Copy,
    Outline,
    Edit,
    ScrollLeft,
    ScrollRight,
    ScrollHome,
    ScrollEnd,
    Quit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key {
    code: KeyCode,
    mods: KeyModifiers,
}

impl Key {
    pub fn from_event(e: KeyEvent) -> Key {
        Key {
            code: e.code,
            mods: e.modifiers & (KeyModifiers::CONTROL | KeyModifiers::ALT),
        }
    }
}

const ACTIONS: KeyTable<Action> = KeyTable {
    down: Action::Down,
    up: Action::Up,
    page_down: Action::PageDown,
    page_up: Action::PageUp,
    half_down: Action::HalfDown,
    half_up: Action::HalfUp,
    top: Action::Top,
    bottom: Action::Bottom,
    next_heading: Action::NextHeading,
    prev_heading: Action::PrevHeading,
    search: Action::Search,
    next_match: Action::NextMatch,
    prev_match: Action::PrevMatch,
    toggle: Action::Toggle,
    next_link: Action::NextLink,
    prev_link: Action::PrevLink,
    open: Action::Open,
    hints: Action::Hints,
    back: Action::Back,
    copy: Action::Copy,
    outline: Action::Outline,
    edit: Action::Edit,
    scroll_left: Action::ScrollLeft,
    scroll_right: Action::ScrollRight,
    scroll_home: Action::ScrollHome,
    scroll_end: Action::ScrollEnd,
    quit: Action::Quit,
};

const MAX_SEQ: usize = 4;

pub struct Keymap {
    bindings: Vec<(Vec<Key>, Action)>,
    pending: Vec<Key>,
}

impl Keymap {
    pub fn new(keys: &Keys) -> Result<Keymap, String> {
        let mut bindings = Vec::new();
        for ((name, specs), (_, &action)) in keys.entries().zip(ACTIONS.entries()) {
            for spec in specs {
                let seq =
                    parse(spec).ok_or_else(|| format!("keys.{name}: invalid key {spec:?}"))?;
                bindings.push((seq, action));
            }
        }
        Ok(Keymap {
            bindings,
            pending: Vec::new(),
        })
    }

    /// Feeds one key press; returns an action once a whole binding has been typed.
    pub fn feed(&mut self, key: Key) -> Option<Action> {
        self.pending.push(key);
        loop {
            if let Some(&(_, action)) = self.bindings.iter().find(|(seq, _)| *seq == self.pending) {
                self.pending.clear();
                return Some(action);
            }
            if self.pending.len() < MAX_SEQ
                && self
                    .bindings
                    .iter()
                    .any(|(seq, _)| seq.starts_with(&self.pending))
            {
                return None;
            }
            if self.pending.len() == 1 {
                self.pending.clear();
                return None;
            }
            self.pending.drain(..self.pending.len() - 1);
        }
    }

    /// First binding of `action`, formatted for the status bar.
    pub fn label(&self, action: Action) -> Option<String> {
        let (seq, _) = self.bindings.iter().find(|(_, a)| *a == action)?;
        Some(seq.iter().map(name).collect())
    }
}

const NAMES: [(&str, KeyCode); 15] = [
    ("up", KeyCode::Up),
    ("down", KeyCode::Down),
    ("left", KeyCode::Left),
    ("right", KeyCode::Right),
    ("pageup", KeyCode::PageUp),
    ("pagedown", KeyCode::PageDown),
    ("home", KeyCode::Home),
    ("end", KeyCode::End),
    ("enter", KeyCode::Enter),
    ("esc", KeyCode::Esc),
    ("tab", KeyCode::Tab),
    ("backtab", KeyCode::BackTab),
    ("backspace", KeyCode::Backspace),
    ("delete", KeyCode::Delete),
    ("space", KeyCode::Char(' ')),
];

fn parse(spec: &str) -> Option<Vec<Key>> {
    let (mods, rest) = modifiers(spec);
    if let Some(&(_, code)) = NAMES.iter().find(|(n, _)| n.eq_ignore_ascii_case(rest)) {
        return Some(vec![Key { code, mods }]);
    }
    if !mods.is_empty() {
        let mut chars = rest.chars();
        let c = chars.next().filter(|_| chars.next().is_none())?;
        return Some(vec![Key {
            code: KeyCode::Char(c.to_ascii_lowercase()),
            mods,
        }]);
    }
    let len = rest.chars().count();
    (1..=MAX_SEQ).contains(&len).then(|| {
        rest.chars()
            .map(|c| Key {
                code: KeyCode::Char(c),
                mods,
            })
            .collect()
    })
}

/// Splits leading `ctrl-` / `alt-` prefixes (case-insensitive) off a key spec.
fn modifiers(mut s: &str) -> (KeyModifiers, &str) {
    let mut mods = KeyModifiers::NONE;
    loop {
        let has = |p: &str| {
            s.len() > p.len() && s.get(..p.len()).is_some_and(|h| h.eq_ignore_ascii_case(p))
        };
        let (m, len) = if has("ctrl-") {
            (KeyModifiers::CONTROL, 5)
        } else if has("alt-") {
            (KeyModifiers::ALT, 4)
        } else {
            return (mods, s);
        };
        mods |= m;
        s = &s[len..];
    }
}

fn name(key: &Key) -> String {
    let mut s = String::new();
    if key.mods.contains(KeyModifiers::CONTROL) {
        s.push_str("C-");
    }
    if key.mods.contains(KeyModifiers::ALT) {
        s.push_str("A-");
    }
    match key.code {
        KeyCode::Char(' ') => s.push_str("space"),
        KeyCode::Char(c) => s.push(c),
        code => s.push_str(
            NAMES
                .iter()
                .find(|(_, k)| *k == code)
                .map_or("?", |(n, _)| n),
        ),
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEventKind;

    fn press(code: KeyCode, mods: KeyModifiers) -> Key {
        Key::from_event(KeyEvent::new_with_kind(code, mods, KeyEventKind::Press))
    }

    fn keymap() -> Keymap {
        Keymap::new(&crate::config::defaults().keys).unwrap()
    }

    #[test]
    fn parses_specs() {
        assert_eq!(
            parse("j"),
            Some(vec![Key {
                code: KeyCode::Char('j'),
                mods: KeyModifiers::NONE
            }])
        );
        assert_eq!(
            parse("ctrl-d"),
            Some(vec![Key {
                code: KeyCode::Char('d'),
                mods: KeyModifiers::CONTROL
            }])
        );
        assert_eq!(parse("PageDown").unwrap()[0].code, KeyCode::PageDown);
        assert_eq!(parse("]]").unwrap().len(), 2);
        assert_eq!(parse("ctrl-"), None);
        assert_eq!(parse(""), None);
        assert_eq!(parse("ctrl-ab"), None);
    }

    #[test]
    fn single_keys_and_shift() {
        let mut k = keymap();
        assert_eq!(
            k.feed(press(KeyCode::Char('j'), KeyModifiers::NONE)),
            Some(Action::Down)
        );
        assert_eq!(
            k.feed(press(KeyCode::Char('G'), KeyModifiers::SHIFT)),
            Some(Action::Bottom)
        );
        assert_eq!(
            k.feed(press(KeyCode::Char('d'), KeyModifiers::CONTROL)),
            Some(Action::HalfDown)
        );
    }

    #[test]
    fn sequences() {
        let mut k = keymap();
        let br = press(KeyCode::Char(']'), KeyModifiers::NONE);
        assert_eq!(k.feed(br), None);
        assert_eq!(k.feed(br), Some(Action::NextHeading));
        assert_eq!(k.feed(br), None);
        assert_eq!(
            k.feed(press(KeyCode::Char('j'), KeyModifiers::NONE)),
            Some(Action::Down)
        );
    }

    #[test]
    fn invalid_spec_is_an_error() {
        let mut keys = crate::config::defaults().keys;
        keys.quit = vec!["ctrl-xyz".into()];
        assert!(Keymap::new(&keys).is_err());
    }

    #[test]
    fn labels() {
        let k = keymap();
        assert_eq!(k.label(Action::Search).as_deref(), Some("/"));
        assert_eq!(k.label(Action::NextHeading).as_deref(), Some("]]"));
    }
}
