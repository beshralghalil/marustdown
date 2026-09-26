use std::ffi::OsStr;
use std::fmt::Write as _;
use std::fs::OpenOptions;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use std::{env, mem, panic};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal;
use unicode_width::UnicodeWidthStr;

use crate::doc::Document;
use crate::keys::{Action, Key, Keymap};
use crate::links::{self, Target};
use crate::render::{self, Decor};
use crate::search::Matcher;
use crate::source::Source;
use crate::theme::{Style, Theme};
use crate::wrap;

const ENTER: &str = "\x1b[?1049h\x1b[?25l\x1b[?7l"; // alt screen, hide cursor, no autowrap
const LEAVE: &str = "\x1b[?7h\x1b[?25h\x1b[?1049l";

const HINTS: [(Action, &str); 8] = [
    (Action::Search, "search"),
    (Action::NextMatch, "next"),
    (Action::Hints, "links"),
    (Action::Toggle, "toggle"),
    (Action::Outline, "outline"),
    (Action::Edit, "edit"),
    (Action::Copy, "copy"),
    (Action::Quit, "quit"),
];

pub fn run(source: Source, path: Option<PathBuf>, theme: &Theme, keys: Keymap) -> io::Result<()> {
    let hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        leave();
        hook(info)
    }));
    terminal::enable_raw_mode()?;
    let _guard = Guard;
    write_tty(ENTER)?;

    let mut pager = Pager::new(source, path, theme, keys, terminal::size()?);
    let mut out = io::stdout().lock();
    let mut size = pager.size;
    loop {
        // Drawing only when idle coalesces key repeats and resize storms.
        if !event::poll(Duration::ZERO)? {
            if size != pager.size {
                pager.resize(size);
            }
            pager.draw(&mut out)?;
        }
        match event::read()? {
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                if pager.key(key) {
                    return Ok(());
                }
            }
            Event::Resize(cols, rows) => size = (cols, rows),
            _ => {}
        }
    }
}

struct Guard;

impl Drop for Guard {
    fn drop(&mut self) {
        leave();
    }
}

fn enter() -> io::Result<()> {
    terminal::enable_raw_mode()?;
    write_tty(ENTER)
}

fn leave() {
    let _ = write_tty(LEAVE);
    let _ = terminal::disable_raw_mode();
}

fn write_tty(s: &str) -> io::Result<()> {
    let mut out = io::stdout();
    out.write_all(s.as_bytes())?;
    out.flush()
}

enum Mode {
    Normal,
    Search(String),
    Outline(Outline),
    Hints(Hints),
}

struct Hints {
    typed: String,
    targets: Vec<Hint>,
}

struct Hint {
    line: usize,
    at: usize,
    id: u16,
    label: String,
}

#[derive(Clone, Copy)]
struct Selection {
    line: usize,
    start: usize,
    end: usize,
    id: u16,
}

/// A file left by following a link, restored by `back`.
struct Page {
    source: Source,
    path: Option<PathBuf>,
    offset: usize,
}

struct Outline {
    query: String,
    items: Vec<usize>, // heading indices matching the query
    sel: usize,
    scroll: usize,
}

struct Pager<'a> {
    source: Source,
    path: Option<PathBuf>,
    theme: &'a Theme,
    keys: Keymap,
    doc: Document,
    size: (u16, u16),
    width: usize,
    margin: usize,
    top: usize,
    cursor: usize,
    matcher: Option<Matcher>,
    selected: Option<Selection>,
    history: Vec<Page>,
    mode: Mode,
    message: Option<String>,
    hints: String,
    osc: String,
    buf: String,
    bar: String,
}

impl<'a> Pager<'a> {
    fn new(
        source: Source,
        path: Option<PathBuf>,
        theme: &'a Theme,
        keys: Keymap,
        size: (u16, u16),
    ) -> Self {
        let mut hints = String::new();
        for (action, label) in HINTS {
            if let Some(key) = keys.label(action) {
                let _ = write!(hints, "{key} {label}  ");
            }
        }
        let mut pager = Pager {
            source,
            path,
            theme,
            keys,
            doc: Document::new(),
            size,
            width: 0,
            margin: 0,
            top: 0,
            cursor: 0,
            matcher: None,
            selected: None,
            history: Vec::new(),
            mode: Mode::Normal,
            message: None,
            hints,
            osc: String::new(),
            buf: String::new(),
            bar: String::new(),
        };
        pager.relayout();
        pager
    }

    fn relayout(&mut self) {
        (self.width, self.margin) = self.theme.layout.fit(self.size.0 as usize);
        self.doc.layout(self.source.text(), self.width, self.theme);
        self.selected = None;
        self.cursor = self.cursor.min(self.doc.len().saturating_sub(1));
        self.follow();
    }

    fn rows(&self) -> usize {
        (self.size.1 as usize)
            .saturating_sub(usize::from(self.theme.layout.status_bar))
            .max(1)
    }

    fn max_top(&self) -> usize {
        self.doc.len().saturating_sub(self.rows())
    }

    fn last(&self) -> usize {
        self.doc.len().saturating_sub(1)
    }

    /// Scrolls just enough to keep the cursor `scroll_off` lines away from the edges.
    fn follow(&mut self) {
        let rows = self.rows();
        let off = self.theme.layout.scroll_off.min(rows.saturating_sub(1) / 2);
        if self.cursor < self.top + off {
            self.top = self.cursor.saturating_sub(off);
        }
        if self.cursor + off >= self.top + rows {
            self.top = self.cursor + off + 1 - rows;
        }
        self.top = self.top.min(self.max_top());
    }

    fn jump(&mut self, line: usize) {
        self.cursor = line.min(self.last());
        self.top = self.cursor.saturating_sub(self.theme.layout.scroll_off);
        self.follow();
    }

    fn resize(&mut self, size: (u16, u16)) {
        let offset = self.doc.source_offset(self.cursor);
        self.size = size;
        self.relayout();
        self.cursor = self.doc.line_at_offset(offset);
        self.follow();
    }

    fn say(&mut self, message: impl Into<String>) {
        self.message = Some(message.into());
    }

    fn section(&self, line: usize) -> Option<usize> {
        self.doc
            .headings
            .partition_point(|h| h.line as usize <= line)
            .checked_sub(1)
    }

    /// Handles one key press; returns true to quit.
    fn key(&mut self, key: KeyEvent) -> bool {
        let ctrl_c =
            key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL);
        match &mut self.mode {
            Mode::Search(query) => {
                match key.code {
                    KeyCode::Enter => {
                        if !query.is_empty() {
                            self.matcher = Some(Matcher::new(query));
                        }
                        self.mode = Mode::Normal;
                        self.search(self.cursor, true);
                    }
                    KeyCode::Backspace => {
                        if query.pop().is_none() {
                            self.mode = Mode::Normal;
                        }
                    }
                    KeyCode::Esc => self.mode = Mode::Normal,
                    _ if ctrl_c => self.mode = Mode::Normal,
                    KeyCode::Char(c) => query.push(c),
                    _ => {}
                }
                false
            }
            Mode::Outline(_) => {
                self.outline_key(key, ctrl_c);
                false
            }
            Mode::Hints(_) => {
                self.hint_key(key, ctrl_c);
                false
            }
            Mode::Normal if ctrl_c => true,
            Mode::Normal => {
                self.message = None;
                self.keys
                    .feed(Key::from_event(key))
                    .is_some_and(|action| self.act(action))
            }
        }
    }

    fn act(&mut self, action: Action) -> bool {
        if !matches!(action, Action::NextLink | Action::PrevLink | Action::Open) {
            self.selected = None;
        }
        let rows = self.rows();
        let move_by = |p: &mut Self, delta: isize| {
            p.cursor = p.cursor.saturating_add_signed(delta).min(p.last());
            p.top = p.top.saturating_add_signed(delta).min(p.max_top());
            p.follow();
        };
        match action {
            Action::Down => {
                self.cursor = (self.cursor + 1).min(self.last());
                self.follow();
            }
            Action::Up => {
                self.cursor = self.cursor.saturating_sub(1);
                self.follow();
            }
            Action::PageDown => move_by(self, rows as isize),
            Action::PageUp => move_by(self, -(rows as isize)),
            Action::HalfDown => move_by(self, (rows / 2).max(1) as isize),
            Action::HalfUp => move_by(self, -((rows / 2).max(1) as isize)),
            Action::Top => self.jump(0),
            Action::Bottom => {
                self.cursor = self.last();
                self.follow();
            }
            Action::NextHeading => {
                let hs = &self.doc.headings;
                if let Some(h) = hs.get(hs.partition_point(|h| h.line as usize <= self.cursor)) {
                    self.jump(h.line as usize);
                }
            }
            Action::PrevHeading => {
                let hs = &self.doc.headings;
                if let Some(k) = hs
                    .partition_point(|h| (h.line as usize) < self.cursor)
                    .checked_sub(1)
                {
                    self.jump(hs[k].line as usize);
                }
            }
            Action::Search => self.mode = Mode::Search(String::new()),
            Action::NextMatch => self.search_next(true),
            Action::PrevMatch => self.search_next(false),
            Action::Toggle => self.toggle(),
            Action::NextLink => self.cycle_link(true),
            Action::PrevLink => self.cycle_link(false),
            Action::Open => match self.selected {
                Some(sel) => self.follow_link(sel.id, false),
                None => self.toggle(),
            },
            Action::Hints => self.start_hints(),
            Action::Back => self.back(),
            Action::Copy => self.copy(),
            Action::Outline => self.open_outline(),
            Action::Edit => self.edit(),
            Action::Quit => return true,
        }
        false
    }

    /// First line from `from` (inclusive) holding a match, wrapping around the end.
    fn search(&mut self, from: usize, forward: bool) {
        let Some(m) = &self.matcher else { return };
        let n = self.doc.len();
        if n == 0 {
            return;
        }
        let hit = |i: &usize| m.find(self.doc.line(*i).0, 0).is_some();
        let found = if forward {
            (from..n)
                .find(hit)
                .map(|i| (i, false))
                .or_else(|| (0..from).find(hit).map(|i| (i, true)))
        } else {
            (0..=from)
                .rev()
                .find(hit)
                .map(|i| (i, false))
                .or_else(|| (from + 1..n).rev().find(hit).map(|i| (i, true)))
        };
        match found {
            Some((line, wrapped)) => {
                self.jump(line);
                if wrapped {
                    self.say("search wrapped");
                }
            }
            None => self.say("pattern not found"),
        }
    }

    fn search_next(&mut self, forward: bool) {
        let n = self.doc.len();
        if n == 0 {
            return;
        }
        let from = if forward {
            (self.cursor + 1) % n
        } else {
            (self.cursor + n - 1) % n
        };
        self.search(from, forward);
    }

    fn copy(&mut self) {
        let blocks = &self.doc.code_blocks;
        let k = blocks.partition_point(|b| (b.last as usize) < self.cursor);
        let pick = match blocks.get(k) {
            Some(b) if (b.first as usize) < self.top + self.rows() => Some(k),
            _ => k.checked_sub(1),
        };
        match pick {
            Some(k) => {
                let code = self.doc.code(k);
                render::clipboard(code, &mut self.osc);
                let lines = code.lines().count();
                self.say(format!(
                    "copied {lines} line{}",
                    if lines == 1 { "" } else { "s" }
                ));
            }
            None => self.say("no code block"),
        }
    }

    fn toggle(&mut self) {
        let Some(task) = self.doc.task_at(self.cursor).copied() else {
            return self.say("no task on this line");
        };
        let mark = if task.done { b' ' } else { b'x' };
        let at = task.offset + 1;
        let result = match &self.path {
            Some(path) => write_mark(path, self.source.base() + at, mark),
            None if self.source.set_byte(at, mark) => Ok(()),
            None => Err(io::Error::other("source is read-only")),
        };
        if let Err(e) = result {
            self.say(format!("toggle failed: {e}"));
        }
        self.reload();
    }

    fn reload(&mut self) {
        if let Some(path) = &self.path {
            match Source::open(path) {
                Ok(source) => self.source = source,
                Err(e) => return self.say(format!("reload failed: {e}")),
            }
        }
        self.relayout();
    }

    fn edit(&mut self) {
        let Some(path) = self.path.clone() else {
            return self.say("stdin has no file to edit");
        };
        let line = line_of(self.source.text(), self.doc.source_offset(self.cursor));
        let editor = env::var("VISUAL")
            .or_else(|_| env::var("EDITOR"))
            .unwrap_or_else(|_| "vi".into());
        let mut words = editor.split_whitespace();
        let program = words.next().unwrap_or("vi");

        leave();
        let status = Command::new(program)
            .args(words)
            .arg(format!("+{line}"))
            .arg(&path)
            .status();
        if let Err(e) = enter() {
            return self.say(format!("terminal: {e}"));
        }
        if let Err(e) = status {
            self.say(format!("{program}: {e}"));
        }

        self.reload();
        self.cursor = self.doc.line_at_offset(offset_of(self.source.text(), line));
        self.follow();
    }

    /// Selects the next (or previous) link after the current selection or cursor,
    /// skipping the wrapped tails of links, and wrapping around the document.
    fn cycle_link(&mut self, forward: bool) {
        let n = self.doc.len();
        if n == 0 {
            return;
        }
        let (from, after) = match self.selected {
            Some(sel) => (sel.line, Some(sel.start)),
            None => (self.cursor, None),
        };
        for step in 0..=n {
            let line = if forward {
                (from + step) % n
            } else {
                (from + n - step) % n
            };
            let doc = &self.doc;
            let spans = doc
                .link_spans(line)
                .enumerate()
                .filter(|&(k, (start, _, id))| {
                    let fresh = k > 0 || !doc.continues_link(line, id);
                    let beyond = step > 0
                        || after.is_none_or(|a| if forward { start > a } else { start < a });
                    fresh && beyond
                });
            let pick = if forward {
                spans.map(|(_, s)| s).next()
            } else {
                spans.map(|(_, s)| s).last()
            };
            if let Some((start, end, id)) = pick {
                self.selected = Some(Selection {
                    line,
                    start,
                    end,
                    id,
                });
                self.cursor = line;
                self.follow();
                return;
            }
        }
        self.say("no links");
    }

    fn start_hints(&mut self) {
        let mut spots: Vec<(usize, usize, u16)> = Vec::new();
        for line in self.top..(self.top + self.rows()).min(self.doc.len()) {
            for (at, _, id) in self.doc.link_spans(line) {
                if !spots.iter().any(|s| s.2 == id) {
                    spots.push((line, at, id));
                }
            }
        }
        if spots.is_empty() {
            return self.say("no links on screen");
        }
        let labels = links::hint_labels(spots.len());
        let targets = spots
            .into_iter()
            .zip(labels)
            .map(|((line, at, id), label)| Hint {
                line,
                at,
                id,
                label,
            })
            .collect();
        self.mode = Mode::Hints(Hints {
            typed: String::new(),
            targets,
        });
    }

    /// Narrows the hint tags; opens the link once one is left, or copies its URL
    /// when the tag was typed in uppercase.
    fn hint_key(&mut self, key: KeyEvent, ctrl_c: bool) {
        let Mode::Hints(mut hints) = mem::replace(&mut self.mode, Mode::Normal) else {
            return;
        };
        match key.code {
            _ if ctrl_c => return,
            KeyCode::Backspace if hints.typed.pop().is_some() => {}
            KeyCode::Char(c) if c.is_ascii_alphabetic() => hints.typed.push(c),
            _ => return,
        }
        let tag = hints.typed.to_ascii_lowercase();
        let copy = hints.typed.chars().any(|c| c.is_ascii_uppercase());
        let mut matching = hints.targets.iter().filter(|t| t.label.starts_with(&tag));
        let first = matching.next().map(|t| t.id);
        match (first, matching.next().is_some()) {
            (None, _) => self.say("no such link"),
            (Some(id), false) => self.follow_link(id, copy),
            _ => self.mode = Mode::Hints(hints),
        }
    }

    fn follow_link(&mut self, id: u16, copy: bool) {
        let Some(url) = (id as usize)
            .checked_sub(1)
            .and_then(|i| self.doc.links.get(i))
            .cloned()
        else {
            return;
        };
        if copy {
            render::clipboard(&url, &mut self.osc);
            return self.say(format!("copied {url}"));
        }
        let base = self.path.as_deref().and_then(Path::parent);
        match links::classify(&url, base) {
            Target::Anchor(anchor) => self.goto_anchor(anchor),
            Target::Local { path, anchor } if links::is_markdown(&path) => self.visit(path, anchor),
            Target::Local { path, .. } => self.external(path.as_os_str()),
            Target::External(url) => self.external(OsStr::new(url)),
        }
    }

    fn goto_anchor(&mut self, anchor: &str) {
        match self.doc.find_anchor(anchor) {
            Some(h) => self.jump(h.line as usize),
            None => self.say(format!("no heading #{anchor}")),
        }
    }

    fn external(&mut self, target: &OsStr) {
        match links::open_external(target) {
            Ok(()) => self.say(format!("opened {}", target.to_string_lossy())),
            Err(e) => self.say(format!("opener: {e}")),
        }
    }

    /// Opens a linked markdown file in place, remembering the current one for `back`.
    fn visit(&mut self, path: PathBuf, anchor: Option<&str>) {
        let source = match Source::open(&path) {
            Ok(source) => source,
            Err(e) => return self.say(format!("{}: {e}", path.display())),
        };
        let offset = self.doc.source_offset(self.cursor);
        let message = path.display().to_string();
        let source = mem::replace(&mut self.source, source);
        let path = self.path.replace(path);
        self.history.push(Page {
            source,
            path,
            offset,
        });
        (self.cursor, self.top) = (0, 0);
        self.relayout();
        match anchor {
            Some(anchor) => self.goto_anchor(anchor),
            None => self.say(message),
        }
    }

    fn back(&mut self) {
        let Some(page) = self.history.pop() else {
            return self.say("no previous file");
        };
        self.source = page.source;
        self.path = page.path;
        self.relayout();
        self.jump(self.doc.line_at_offset(page.offset));
    }

    fn open_outline(&mut self) {
        if self.doc.headings.is_empty() {
            return self.say("no headings");
        }
        let mut outline = Outline {
            query: String::new(),
            items: Vec::new(),
            sel: 0,
            scroll: 0,
        };
        self.filter_outline(&mut outline);
        outline.sel = self.section(self.cursor).unwrap_or(0);
        self.mode = Mode::Outline(outline);
    }

    fn filter_outline(&self, outline: &mut Outline) {
        let m = Matcher::new(&outline.query);
        outline.items.clear();
        outline
            .items
            .extend((0..self.doc.headings.len()).filter(|&i| {
                outline.query.is_empty()
                    || m.find(self.doc.title(&self.doc.headings[i]), 0).is_some()
            }));
        outline.sel = outline.sel.min(outline.items.len().saturating_sub(1));
    }

    fn outline_key(&mut self, key: KeyEvent, ctrl_c: bool) {
        let Mode::Outline(mut o) = mem::replace(&mut self.mode, Mode::Normal) else {
            return;
        };
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let page = self.rows();
        let last = o.items.len().saturating_sub(1);
        match key.code {
            KeyCode::Esc => return,
            _ if ctrl_c => return,
            KeyCode::Enter => {
                if let Some(&h) = o.items.get(o.sel) {
                    self.jump(self.doc.headings[h].line as usize);
                }
                return;
            }
            KeyCode::Up => o.sel = o.sel.saturating_sub(1),
            KeyCode::Char('p' | 'k') if ctrl => o.sel = o.sel.saturating_sub(1),
            KeyCode::Down | KeyCode::Tab => o.sel = (o.sel + 1).min(last),
            KeyCode::Char('n' | 'j') if ctrl => o.sel = (o.sel + 1).min(last),
            KeyCode::PageUp => o.sel = o.sel.saturating_sub(page),
            KeyCode::PageDown => o.sel = (o.sel + page).min(last),
            KeyCode::Backspace => {
                o.query.pop();
                self.filter_outline(&mut o);
            }
            KeyCode::Char(c) if !ctrl => {
                o.query.push(c);
                o.sel = 0;
                self.filter_outline(&mut o);
            }
            _ => {}
        }
        self.mode = Mode::Outline(o);
    }

    fn draw(&mut self, out: &mut impl Write) -> io::Result<()> {
        let mut buf = mem::take(&mut self.buf);
        buf.clear();
        buf.push_str(&self.osc);
        self.osc.clear();
        buf.push_str("\x1b[?2026h");
        if let Mode::Outline(o) = &mut self.mode {
            let rows = (self.size.1 as usize).saturating_sub(1).max(1);
            o.scroll = o.scroll.min(o.sel).max((o.sel + 1).saturating_sub(rows));
        }
        match &self.mode {
            Mode::Outline(o) => self.draw_outline(o, &mut buf),
            _ => self.draw_lines(&mut buf),
        }
        if self.theme.layout.status_bar || !matches!(self.mode, Mode::Normal) {
            self.draw_status(&mut buf);
        } else {
            buf.push_str("\x1b[?25l");
        }
        buf.push_str("\x1b[?2026l");
        let result = out.write_all(buf.as_bytes()).and_then(|()| out.flush());
        self.buf = buf;
        result
    }

    fn draw_lines(&self, buf: &mut String) {
        let (t, cursor_style) = (self.theme, self.theme.styles.cursor);
        let mut labels = Vec::new();
        let hints = match &self.mode {
            Mode::Hints(h) => Some((h, h.typed.to_ascii_lowercase())),
            _ => None,
        };
        for row in 0..self.rows() {
            let _ = write!(buf, "\x1b[{};1H", row + 1);
            let i = self.top + row;
            if i < self.doc.len() {
                labels.clear();
                if let Some((h, tag)) = &hints {
                    labels.extend(
                        h.targets
                            .iter()
                            .filter(|t| t.line == i && t.label.starts_with(tag.as_str()))
                            .map(|t| (t.at, &t.label[tag.len()..])),
                    );
                }
                let decor = Decor {
                    under: if i == self.cursor {
                        cursor_style
                    } else {
                        Style::default()
                    },
                    search: self.matcher.as_ref(),
                    select: self
                        .selected
                        .filter(|s| s.line == i)
                        .map(|s| (s.start, s.end)),
                    labels: &labels,
                };
                render::pad(buf, self.margin);
                render::line(&self.doc, i, t, &decor, buf);
                if i == self.cursor {
                    let used = self.doc.line(i).0.width();
                    render::blank(t, cursor_style, self.width.saturating_sub(used), buf);
                }
            }
            buf.push_str("\x1b[K");
        }
    }

    fn draw_outline(&self, o: &Outline, buf: &mut String) {
        let (t, s) = (self.theme, &self.theme.styles);
        let rows = (self.size.1 as usize).saturating_sub(1).max(1);
        for row in 0..rows {
            let _ = write!(buf, "\x1b[{};1H", row + 1);
            if let Some(&h) = o.items.get(o.scroll + row) {
                let heading = &self.doc.headings[h];
                let under = if o.scroll + row == o.sel {
                    s.cursor
                } else {
                    Style::default()
                };
                let indent = 2 * (heading.level as usize - 1);
                let line_w = digits(heading.line as usize + 1);
                let title = self.doc.title(heading);
                let (cut, title_w) =
                    wrap::fit(title, self.width.saturating_sub(indent + line_w + 2));
                render::pad(buf, self.margin);
                render::blank(t, under, indent, buf);
                render::styled(
                    t,
                    under.over(s.heading(heading.level as usize)),
                    &title[..cut],
                    buf,
                );
                render::blank(
                    t,
                    under,
                    self.width.saturating_sub(indent + title_w + line_w),
                    buf,
                );
                let mut digits_buf = [0; 20];
                render::styled(
                    t,
                    under.over(s.outline_level),
                    number(heading.line as usize + 1, &mut digits_buf),
                    buf,
                );
            }
            buf.push_str("\x1b[K");
        }
    }

    fn draw_status(&mut self, buf: &mut String) {
        let cols = self.size.0 as usize;
        let mut bar = mem::take(&mut self.bar);
        bar.clear();
        let prompt = match &self.mode {
            Mode::Search(q) => Some(("/", q.as_str())),
            Mode::Outline(o) => Some(("outline: ", o.query.as_str())),
            Mode::Hints(h) => Some(("link (uppercase copies): ", h.typed.as_str())),
            Mode::Normal => None,
        };
        match prompt {
            Some((label, query)) => {
                bar.push(' ');
                bar.push_str(label);
                bar.push_str(query);
            }
            None => {
                let _ = write!(bar, " ln {}/{}", self.cursor + 1, self.doc.len());
                self.breadcrumb(&mut bar);
            }
        }
        let cursor_col = bar.width() + 1;
        let right = self.message.as_deref().unwrap_or(&self.hints).trim_end();
        let used = bar.width();
        if prompt.is_none() && used + right.width() + 3 <= cols {
            render::pad(&mut bar, cols - used - right.width() - 1);
            bar.push_str(right);
        }
        let (fit, w) = wrap::fit(&bar, cols);
        render::pad(&mut bar, cols.saturating_sub(w));
        let _ = write!(buf, "\x1b[{};1H", self.size.1);
        render::styled(
            self.theme,
            self.theme.styles.status,
            &bar[..fit + cols.saturating_sub(w)],
            buf,
        );
        if prompt.is_some() {
            let _ = write!(
                buf,
                "\x1b[{};{}H\x1b[?25h",
                self.size.1,
                cursor_col.min(cols)
            );
        } else {
            buf.push_str("\x1b[?25l");
        }
        self.bar = bar;
    }

    fn breadcrumb(&self, bar: &mut String) {
        let Some(mut h) = self.section(self.cursor) else {
            return;
        };
        let g = &self.theme.glyphs;
        let mut chain = [0usize; 6];
        let mut n = 0;
        loop {
            chain[n] = h;
            n += 1;
            match self.doc.headings[h].parent {
                Some(p) if n < chain.len() => h = p as usize,
                _ => break,
            }
        }
        bar.push_str(&g.separator);
        for (k, &i) in chain[..n].iter().rev().enumerate() {
            if k > 0 {
                bar.push_str(&g.breadcrumb);
            }
            bar.push_str(self.doc.title(&self.doc.headings[i]));
        }
    }
}

/// 1-based source line holding byte `offset`.
fn line_of(text: &str, offset: usize) -> usize {
    text.as_bytes()[..offset.min(text.len())]
        .iter()
        .filter(|&&b| b == b'\n')
        .count()
        + 1
}

/// Byte offset where 1-based `line` starts, clamped to the end of `text`.
fn offset_of(text: &str, line: usize) -> usize {
    match line.checked_sub(2) {
        None => 0,
        Some(n) => text
            .match_indices('\n')
            .nth(n)
            .map_or(text.len(), |(i, _)| i + 1),
    }
}

fn digits(n: usize) -> usize {
    n.checked_ilog10().map_or(1, |d| d as usize + 1)
}

/// Formats `n` into `buf` without allocating.
fn number(mut n: usize, buf: &mut [u8; 20]) -> &str {
    let mut i = buf.len();
    loop {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    std::str::from_utf8(&buf[i..]).expect("ASCII digits")
}

/// Flips the task mark at byte `at` of `path`, after checking the file still has a
/// task box there (it may have changed since it was loaded).
fn write_mark(path: &Path, at: usize, mark: u8) -> io::Result<()> {
    let mut file = OpenOptions::new().read(true).write(true).open(path)?;
    let mut current = [0; 3];
    file.seek(SeekFrom::Start(at as u64 - 1))?;
    file.read_exact(&mut current)?;
    if !matches!(&current, b"[ ]" | b"[x]" | b"[X]") {
        return Err(io::Error::other("file changed on disk; reloaded"));
    }
    file.seek(SeekFrom::Start(at as u64))?;
    file.write_all(&[mark])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_offset_round_trip() {
        let text = "a\nbb\n\nc";
        assert_eq!(line_of(text, 0), 1);
        assert_eq!(line_of(text, 3), 2);
        assert_eq!(line_of(text, 7), 4);
        assert_eq!(offset_of(text, 1), 0);
        assert_eq!(offset_of(text, 2), 2);
        assert_eq!(offset_of(text, 4), 6);
        assert_eq!(offset_of(text, 9), text.len());
    }

    #[test]
    fn formats_numbers() {
        let mut buf = [0; 20];
        assert_eq!(number(0, &mut buf), "0");
        assert_eq!(number(40213, &mut buf), "40213");
        assert_eq!(digits(40213), 5);
    }
}
