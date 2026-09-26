use std::fmt::Write as _;
use std::io;

use crate::doc::Document;
use crate::search::Matcher;
use crate::theme::{BOLD, DIM, ITALIC, Paint, REVERSE, STRIKE, Style, Theme, UNDERLINE};

const ATTRS: [(u8, &str); 6] = [
    (BOLD, "1"),
    (DIM, "2"),
    (ITALIC, "3"),
    (UNDERLINE, "4"),
    (REVERSE, "7"),
    (STRIKE, "9"),
];
const OSC8_CLOSE: &str = "\x1b]8;;\x1b\\";
const CHUNK: usize = 1 << 16;

/// Writes line `i` as ANSI text. `under` sits beneath every run (the cursor line),
/// and search hits are split out of their runs here, never stored.
pub fn line(
    doc: &Document,
    i: usize,
    theme: &Theme,
    search: Option<&Matcher>,
    under: Style,
    out: &mut String,
) {
    let (text, runs) = doc.line(i);
    let mut pen = Pen::new(theme, &doc.links, out);
    let mut hits = search.into_iter().flat_map(|m| m.matches(text)).peekable();
    for (k, run) in runs.iter().enumerate() {
        let end = runs.get(k + 1).map_or(text.len(), |n| n.at as usize);
        let style = under.over(run.style);
        let mut pos = run.at as usize;
        while pos < end {
            while hits.next_if(|&(_, e)| e <= pos).is_some() {}
            let (stop, style) = match hits.peek() {
                Some(&(s, e)) if s <= pos => (e.min(end), style.over(theme.styles.search)),
                Some(&(s, _)) if s < end => (s, style),
                _ => (end, style),
            };
            pen.put(&text[pos..stop], style);
            pos = stop;
        }
    }
    pen.finish();
}

/// Writes `s` in `style` followed by a reset.
pub fn styled(theme: &Theme, style: Style, s: &str, out: &mut String) {
    let mut pen = Pen::new(theme, &[], out);
    pen.put(s, style);
    pen.finish();
}

/// Streams the whole document in bounded chunks.
pub fn cat(
    doc: &Document,
    theme: &Theme,
    margin: usize,
    out: &mut impl io::Write,
) -> io::Result<()> {
    let mut buf = String::with_capacity(CHUNK + 4096);
    for i in 0..doc.len() {
        if !doc.line(i).0.is_empty() {
            pad(&mut buf, margin);
            line(doc, i, theme, None, Style::default(), &mut buf);
        }
        buf.push('\n');
        if buf.len() >= CHUNK {
            out.write_all(buf.as_bytes())?;
            buf.clear();
        }
    }
    out.write_all(buf.as_bytes())?;
    out.flush()
}

/// `n` blank columns in `style` (for backgrounds that run to the edge).
pub fn blank(theme: &Theme, style: Style, n: usize, out: &mut String) {
    let mut pen = Pen::new(theme, &[], out);
    pen.sgr(style);
    pad(pen.out, n);
    pen.finish();
}

pub fn pad(out: &mut String, n: usize) {
    out.extend(std::iter::repeat_n(' ', n));
}

/// OSC 52 sequence that puts `text` on the system clipboard (works over SSH).
pub fn clipboard(text: &str, out: &mut String) {
    out.push_str("\x1b]52;c;");
    base64(text.as_bytes(), out);
    out.push('\x07');
}

fn base64(data: &[u8], out: &mut String) {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    for chunk in data.chunks(3) {
        let b = |i: usize| chunk.get(i).copied().unwrap_or(0) as u32;
        let n = (b(0) << 16) | (b(1) << 8) | b(2);
        let sextet = |shift: u32| TABLE[(n >> shift & 63) as usize] as char;
        out.push(sextet(18));
        out.push(sextet(12));
        out.push(if chunk.len() > 1 { sextet(6) } else { '=' });
        out.push(if chunk.len() > 2 { sextet(0) } else { '=' });
    }
}

/// Tracks the terminal's current style so only changes are emitted.
struct Pen<'a> {
    theme: &'a Theme,
    links: &'a [String],
    out: &'a mut String,
    style: Style,
    link: u16,
}

impl<'a> Pen<'a> {
    fn new(theme: &'a Theme, links: &'a [String], out: &'a mut String) -> Self {
        Pen {
            theme,
            links,
            out,
            style: Style::default(),
            link: 0,
        }
    }

    fn put(&mut self, s: &str, style: Style) {
        if style.link != self.link {
            if self.link != 0 {
                self.out.push_str(OSC8_CLOSE);
            }
            if let Some(url) = (style.link as usize)
                .checked_sub(1)
                .and_then(|i| self.links.get(i))
            {
                let _ = write!(self.out, "\x1b]8;id={};{url}\x1b\\", style.link);
            }
            self.link = style.link;
        }
        self.sgr(style);
        self.out.push_str(s);
    }

    fn sgr(&mut self, new: Style) {
        let t = self.theme;
        let mut old = std::mem::replace(&mut self.style, new);
        let reset = old.attrs & !new.attrs != 0;
        if reset {
            old = Style::default();
        }
        let (fg, bg) = (t.paint(new.fg), t.paint(new.bg));
        let fg_changed = fg != t.paint(old.fg);
        let bg_changed = bg != t.paint(old.bg);
        let added = new.attrs & !old.attrs;
        if !reset && added == 0 && !fg_changed && !bg_changed {
            return;
        }
        let out = &mut *self.out;
        out.push_str("\x1b[");
        let mut first = true;
        let mut sep = |out: &mut String| {
            if !std::mem::replace(&mut first, false) {
                out.push(';');
            }
        };
        if reset {
            sep(out);
            out.push('0');
        }
        for (bit, code) in ATTRS {
            if added & bit != 0 {
                sep(out);
                out.push_str(code);
            }
        }
        if fg_changed {
            sep(out);
            color(out, fg, 30);
        }
        if bg_changed {
            sep(out);
            color(out, bg, 40);
        }
        out.push('m');
    }

    fn finish(self) {
        if self.link != 0 {
            self.out.push_str(OSC8_CLOSE);
        }
        let t = self.theme;
        if self.style.attrs != 0
            || t.paint(self.style.fg).is_some()
            || t.paint(self.style.bg).is_some()
        {
            self.out.push_str("\x1b[0m");
        }
    }
}

/// SGR color parameter; `base` is 30 for foreground, 40 for background.
fn color(out: &mut String, paint: Option<Paint>, base: u8) {
    match paint {
        None => push_u8(out, base + 9),
        Some(Paint::Indexed(n)) if n < 8 => push_u8(out, base + n),
        Some(Paint::Indexed(n)) if n < 16 => push_u8(out, base + 60 + n - 8),
        Some(Paint::Indexed(n)) => {
            push_u8(out, base + 8);
            out.push_str(";5;");
            push_u8(out, n);
        }
        Some(Paint::Rgb(r, g, b)) => {
            push_u8(out, base + 8);
            out.push_str(";2;");
            for (i, c) in [r, g, b].into_iter().enumerate() {
                if i > 0 {
                    out.push(';');
                }
                push_u8(out, c);
            }
        }
    }
}

fn push_u8(out: &mut String, n: u8) {
    if n >= 100 {
        out.push((b'0' + n / 100) as char);
    }
    if n >= 10 {
        out.push((b'0' + n / 10 % 10) as char);
    }
    out.push((b'0' + n % 10) as char);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::test_theme;

    fn b64(s: &str) -> String {
        let mut out = String::new();
        base64(s.as_bytes(), &mut out);
        out
    }

    #[test]
    fn base64_padding() {
        assert_eq!(b64(""), "");
        assert_eq!(b64("f"), "Zg==");
        assert_eq!(b64("fo"), "Zm8=");
        assert_eq!(b64("foo"), "Zm9v");
        assert_eq!(b64("foobar"), "Zm9vYmFy");
    }

    fn render(src: &str, search: Option<&Matcher>) -> String {
        let theme = test_theme();
        let mut doc = Document::new();
        doc.layout(src, 80, &theme);
        let mut out = String::new();
        line(&doc, 0, &theme, search, Style::default(), &mut out);
        out
    }

    #[test]
    fn emits_only_changes() {
        let out = render("a **b** *c*", None);
        assert_eq!(out, "a \x1b[1mb\x1b[0m \x1b[3mc\x1b[0m");
    }

    #[test]
    fn plain_text_has_no_escapes() {
        assert_eq!(render("hello", None), "hello");
    }

    #[test]
    fn links_use_osc8() {
        let out = render("[x](http://a)", None);
        assert!(out.contains("\x1b]8;id=1;http://a\x1b\\"));
        assert!(out.contains(OSC8_CLOSE));
    }

    #[test]
    fn search_hits_split_runs() {
        let out = render("abcabc", Some(&Matcher::new("bc")));
        assert_eq!(out.matches("\x1b[1;7;").count(), 2);
        assert!(out.starts_with('a'));
    }

    #[test]
    fn color_codes() {
        let mut s = String::new();
        color(&mut s, Some(Paint::Indexed(3)), 30);
        s.push(' ');
        color(&mut s, Some(Paint::Indexed(12)), 40);
        s.push(' ');
        color(&mut s, Some(Paint::Indexed(200)), 30);
        s.push(' ');
        color(&mut s, Some(Paint::Rgb(1, 22, 255)), 40);
        s.push(' ');
        color(&mut s, None, 30);
        assert_eq!(s, "33 104 38;5;200 48;2;1;22;255 39");
    }
}
