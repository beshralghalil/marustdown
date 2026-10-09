use std::fmt::Write as _;
use std::io;

use unicode_width::UnicodeWidthChar;

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

/// Per-frame decorations layered over a line; none of them are stored in the document.
#[derive(Default)]
pub struct Decor<'a> {
    pub under: Style,                   // beneath every run (the cursor line)
    pub search: Option<&'a Matcher>,    // hits split out of their runs
    pub select: Option<(usize, usize)>, // byte range of the selected link
    pub labels: &'a [(usize, &'a str)], // hint tags inserted before these offsets, sorted
    pub window: Option<Window>,         // the visible part of a wide line
    pub plain: bool,                    // text only, no escape sequences
    pub holes: &'a [(usize, usize)],    // column ranges left untouched, for pictures
}

/// The visible part of a wide line: bytes `start..end` scroll, and `view` of their
/// `width` columns are shown from column `offset`.
#[derive(Clone, Copy, Debug)]
pub struct Window {
    pub start: usize,
    pub end: usize,
    pub offset: usize,
    pub view: usize,
    pub width: usize,
}

impl Window {
    /// Window of `line`, part of wide block `k`, scrolled to `offset` (clamped).
    pub fn new(doc: &Document, k: usize, line: usize, offset: usize) -> Self {
        let (start, end) = doc.window(k, line);
        let (view, width) = (doc.wides[k].view as usize, doc.wides[k].width as usize);
        let offset = offset.min(width.saturating_sub(view));
        Window {
            start,
            end,
            offset,
            view,
            width,
        }
    }
}

/// Writes line `i` as ANSI text.
pub fn line(doc: &Document, i: usize, theme: &Theme, decor: &Decor, out: &mut String) {
    let (text, runs) = doc.line(i);
    let s = &theme.styles;
    let mut pen = Pen::new(theme, &doc.links, out);
    pen.plain = decor.plain;
    pen.holes = decor.holes;
    let mut hits = decor
        .search
        .into_iter()
        .flat_map(|m| m.matches(text))
        .peekable();
    let mut labels = decor.labels.iter().peekable();
    let g = &theme.glyphs;
    let mut clip = Clip {
        window: decor.window,
        col: 0,
        marks: [&g.overflow_left, &g.overflow_right],
    };
    for (k, run) in runs.iter().enumerate() {
        let at = run.at as usize;
        let end = runs.get(k + 1).map_or(text.len(), |n| n.at as usize);
        while let Some(&(pos, label)) = labels.next_if(|l| l.0 <= at) {
            if pos == at {
                pen.put(label, decor.under.over(s.hint));
            }
        }
        let mut style = decor.under.over(run.style);
        if decor.select.is_some_and(|(a, b)| (a..b).contains(&at)) {
            style = style.over(s.link_selected);
        }
        let mut pos = at;
        while pos < end {
            while hits.next_if(|&(_, e)| e <= pos).is_some() {}
            let (stop, style) = match hits.peek() {
                Some(&(s0, e)) if s0 <= pos => (e.min(end), style.over(s.search)),
                Some(&(s0, _)) if s0 < end => (s0, style),
                _ => (end, style),
            };
            clip.put(&mut pen, pos, &text[pos..stop], style, s.overflow);
            pos = stop;
        }
    }
    pen.finish();
}

/// Crops the scrolling part of a wide line to its window, marking hidden content at
/// the cut edges.
struct Clip<'a> {
    window: Option<Window>,
    col: usize, // columns of the scrolling part written so far
    marks: [&'a str; 2],
}

impl Clip<'_> {
    fn put(&mut self, pen: &mut Pen, at: usize, s: &str, style: Style, mark: Style) {
        let Some(w) = self.window.filter(|w| at < w.end && w.start < at + s.len()) else {
            return pen.put(s, style);
        };
        let (a, b) = (w.start.saturating_sub(at), (w.end - at).min(s.len()));
        pen.put(&s[..a], style);
        let (lo, hi) = (w.offset, w.offset + w.view);
        let (more_left, more_right) = (lo > 0, hi < w.width);
        let mut buf = [0; 4];
        for c in s[a..b].chars() {
            let cw = c.width().unwrap_or(0);
            let c0 = self.col;
            self.col += cw;
            let (from, to) = (c0.max(lo), (c0 + cw).min(hi));
            if from >= to && !(cw == 0 && (lo..hi).contains(&c0)) {
                continue;
            }
            let (left_edge, right_edge) = (from == lo && more_left, to == hi && more_right);
            if left_edge || right_edge || from > c0 || to < c0 + cw {
                for col in from..to {
                    match (col == lo && left_edge, col + 1 == hi && right_edge) {
                        (true, _) => pen.put(self.marks[0], style.over(mark)),
                        (_, true) => pen.put(self.marks[1], style.over(mark)),
                        _ => pen.put(" ", style),
                    }
                }
            } else {
                pen.put(c.encode_utf8(&mut buf), style);
            }
        }
        pen.put(&s[b..], style);
    }
}

/// Writes `s` in `style` followed by a reset.
pub fn styled(theme: &Theme, style: Style, s: &str, out: &mut String) {
    let mut pen = Pen::new(theme, &[], out);
    pen.put(s, style);
    pen.finish();
}

/// Streams the whole document in bounded chunks; `plain` writes bare text with no
/// escape sequences at all, for files and pipes.
pub fn cat(
    doc: &Document,
    theme: &Theme,
    margin: usize,
    plain: bool,
    out: &mut impl io::Write,
) -> io::Result<()> {
    let mut buf = String::with_capacity(CHUNK + 4096);
    for i in 0..doc.len() {
        let text = doc.line(i).0;
        if !text.trim_end().is_empty() || (!plain && !text.is_empty()) {
            pad(&mut buf, margin);
            let window = doc.wide_at(i).map(|k| Window::new(doc, k, i, 0));
            line(
                doc,
                i,
                theme,
                &Decor {
                    window,
                    plain,
                    ..Decor::default()
                },
                &mut buf,
            );
            if plain {
                buf.truncate(buf.trim_end_matches(' ').len());
            }
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

pub fn base64(data: &[u8], out: &mut String) {
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
    plain: bool,
    holes: &'a [(usize, usize)],
    col: usize,  // columns put so far, tracked only with holes
    skip: usize, // columns of a hole still to move past
}

impl<'a> Pen<'a> {
    fn new(theme: &'a Theme, links: &'a [String], out: &'a mut String) -> Self {
        Pen {
            theme,
            links,
            out,
            style: Style::default(),
            link: 0,
            plain: false,
            holes: &[],
            col: 0,
            skip: 0,
        }
    }

    /// Writes `s`, moving the cursor over the columns that fall in holes.
    fn put(&mut self, s: &str, style: Style) {
        if self.holes.is_empty() {
            return self.write(s, style);
        }
        let mut from = 0;
        for (i, c) in s.char_indices() {
            let col = self.col;
            self.col += c.width().unwrap_or(0);
            if self.holes.iter().any(|&(a, b)| (a..b).contains(&col)) {
                self.write(&s[from..i], style);
                self.skip += self.col - col;
                from = i + c.len_utf8();
            } else {
                self.step();
            }
        }
        self.write(&s[from..], style);
    }

    fn step(&mut self) {
        if self.skip > 0 {
            let _ = write!(self.out, "\x1b[{}C", std::mem::take(&mut self.skip));
        }
    }

    fn write(&mut self, s: &str, style: Style) {
        if s.is_empty() {
            return;
        }
        if self.plain {
            return self.out.push_str(s);
        }
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

    fn finish(mut self) {
        self.step();
        if self.plain {
            return;
        }
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
        line(
            &doc,
            0,
            &theme,
            &Decor {
                search,
                ..Decor::default()
            },
            &mut out,
        );
        out
    }

    #[test]
    fn holes_are_stepped_over() {
        let theme = test_theme();
        let mut doc = Document::new();
        doc.layout("abc日ef", 80, &theme);
        let hole = |holes: &[(usize, usize)]| {
            let mut out = String::new();
            let decor = Decor {
                holes,
                ..Decor::default()
            };
            line(&doc, 0, &theme, &decor, &mut out);
            out
        };
        assert_eq!(hole(&[(1, 3)]), "a\x1b[2C日ef");
        assert_eq!(hole(&[(3, 4)]), "abc\x1b[2Cef");
        assert_eq!(hole(&[(0, 1), (6, 8)]), "\x1b[1Cbc日e\x1b[1C");
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
    fn labels_and_selection() {
        let theme = test_theme();
        let mut doc = Document::new();
        doc.layout("go [here](u) now", 80, &theme);
        let (start, end, _) = doc.link_spans(0).next().unwrap();
        let mut out = String::new();
        let decor = Decor {
            select: Some((start, end)),
            labels: &[(start, "a")],
            ..Decor::default()
        };
        line(&doc, 0, &theme, &decor, &mut out);
        let plain: String = strip(&out);
        assert_eq!(plain, "go ahere↗ now");
        assert!(out.contains("\x1b[1;7;"));
    }

    fn windowed(src: &str, width: usize, row: usize, offset: usize) -> String {
        let theme = test_theme();
        let mut doc = Document::new();
        doc.layout(src, width, &theme);
        let k = doc.wide_at(row).expect("wide line");
        let window = Some(Window::new(&doc, k, row, offset));
        let mut out = String::new();
        line(
            &doc,
            row,
            &theme,
            &Decor {
                window,
                ..Decor::default()
            },
            &mut out,
        );
        strip(&out)
    }

    #[test]
    fn plain_cat_clips_wide_blocks() {
        let theme = test_theme();
        let mut doc = Document::new();
        doc.layout("```\nabcdefghijklmnopqrstuvwxyz\n```", 20, &theme);
        let mut out = Vec::new();
        cat(&doc, &theme, 0, true, &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert_eq!(text.lines().nth(1), Some("│ 1  abcdefghijkl› │"));
        assert!(!text.contains('\x1b'));
    }

    #[test]
    fn window_crops_and_marks_hidden_content() {
        let src = "```\nabcdefghijklmnopqrstuvwxyz\n```";
        assert_eq!(windowed(src, 20, 1, 0), "│ 1  abcdefghijkl› │");
        assert_eq!(windowed(src, 20, 1, 5), "│ 1  ‹ghijklmnopq› │");
        assert_eq!(windowed(src, 20, 1, 99), "│ 1  ‹opqrstuvwxyz │");
    }

    #[test]
    fn window_keeps_columns_across_wide_characters() {
        let src = "```\n日本語日本語日本語日本語\n```";
        for offset in 0..12 {
            let out = windowed(src, 20, 1, offset);
            assert_eq!(
                unicode_width::UnicodeWidthStr::width(out.as_str()),
                20,
                "offset {offset}: {out}"
            );
        }
    }

    fn strip(s: &str) -> String {
        let mut out = String::new();
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '\x1b' if chars.peek() == Some(&'[') => {
                    while chars.next().is_some_and(|c| c != 'm') {}
                }
                '\x1b' => while chars.next().is_some_and(|c| c != '\\') {},
                c => out.push(c),
            }
        }
        out
    }

    #[test]
    fn plain_cat_has_no_escapes() {
        let theme = test_theme();
        let mut doc = Document::new();
        doc.layout(
            "# Title\n\n**bold** [link](u) `code`\n\n- [ ] task",
            30,
            &theme,
        );
        let mut out = Vec::new();
        cat(&doc, &theme, 0, true, &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(!text.contains('\x1b'));
        assert!(text.starts_with("██ Title\n"));
        assert!(text.contains("bold link↗  code"));
        assert!(text.lines().all(|l| l == l.trim_end()));
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
