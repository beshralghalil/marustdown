use std::collections::{HashMap, VecDeque};
use std::fmt::Write as _;
use std::mem;
use std::ops::Range;

use pulldown_cmark::{
    Alignment, BlockQuoteKind, CodeBlockKind, Event, LinkType, OffsetIter, Options, Parser, Tag,
    TagEnd,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::blocks::{self, Context};
use crate::config::Styles;
use crate::doc::{Document, Picture, Task};
use crate::highlight::{self, Span, Token};
#[cfg(feature = "math")]
use crate::math;
use crate::theme::{Style, Theme};
use crate::{links, table, wrap};

const OPTIONS: Options = Options::ENABLE_TABLES
    .union(Options::ENABLE_STRIKETHROUGH)
    .union(Options::ENABLE_TASKLISTS)
    .union(Options::ENABLE_GFM);

/// Columns kept of a scrollable line; the rest is dropped.
const MAX_WIDE: usize = 4096;

/// Work that outlives a layout: highlighted and rendered blocks, so re-layouts only redo
/// what changed.
#[derive(Default)]
pub struct Cache {
    syntax: highlight::Cache,
    blocks: blocks::Cache,
}

/// The cells an image takes when drawn as a picture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fit {
    pub id: u32,
    pub cols: u16,
    pub rows: u16,
}

/// Sizes images for the layout, so an image alone in its paragraph becomes a picture.
pub trait Pictures {
    /// The picture for the image at `url` in at most `cols` columns; `None` keeps its
    /// alt text.
    fn fit(&mut self, url: &str, cols: usize) -> Option<Fit>;
}

/// Lays `src` out into `doc` at `width` columns.
pub fn build(
    doc: &mut Document,
    cache: &mut Cache,
    src: &str,
    width: usize,
    theme: &Theme,
    pictures: Option<&mut dyn Pictures>,
) {
    let pictures = pictures.map(|p| p as &mut dyn Pictures);
    Builder::new(doc, cache, theme, width, pictures).run(src);
    cache.syntax.truncate(doc.code_blocks.len());
}

/// Inline scratch: text plus style runs with absolute offsets, reused between blocks.
#[derive(Default)]
struct Inline {
    text: String,
    runs: Vec<(usize, Style)>,
}

impl Inline {
    fn push(&mut self, s: &str, style: Style) {
        if s.is_empty() {
            return;
        }
        self.mark(style);
        self.text.push_str(s);
    }

    fn push_clean(&mut self, s: &str, style: Style) {
        if !needs_clean(s) {
            return self.push(s, style);
        }
        self.mark(style);
        self.text.extend(s.chars().map(|c| match c {
            c if c.is_whitespace() && c.is_control() => ' ',
            c if c.is_control() => '\u{fffd}',
            c => c,
        }));
    }

    fn mark(&mut self, style: Style) {
        if self.runs.last().is_none_or(|r| r.1 != style) {
            self.runs.push((self.text.len(), style));
        }
    }

    fn clear(&mut self) {
        self.text.clear();
        self.runs.clear();
    }
}

/// True if `s` may hold C0/C1 control characters, which must never reach the terminal.
fn needs_clean(s: &str) -> bool {
    s.bytes().any(|b| b < 0x20 || b == 0x7f || b == 0xc2)
}

/// Copies inline text `a..b` into the current line, layering its runs over `base`.
fn emit(doc: &mut Document, inline: &Inline, a: usize, b: usize, base: Style) {
    let runs = &inline.runs;
    let mut k = runs.partition_point(|r| r.0 <= a).saturating_sub(1);
    let mut pos = a;
    while pos < b {
        let next = runs.get(k + 1).map_or(b, |r| r.0.min(b));
        doc.push(&inline.text[pos..next], base.over(runs[k].1));
        pos = next;
        k += 1;
    }
}

#[derive(Clone, Copy)]
struct Frame {
    start: usize, // marker range in Out::markers
    end: usize,
    style: Style,
    width: usize,
    hanging: bool, // continuation lines get blanks instead of the marker
    started: bool,
}

/// Line writer that prefixes every line with the open quote and list frames.
struct Out<'d> {
    doc: &'d mut Document,
    frames: Vec<Frame>,
    markers: String,
    width: usize,
    indent: usize,
}

impl Out<'_> {
    fn avail(&self) -> usize {
        self.width.saturating_sub(self.indent).max(1)
    }

    fn line(&self) -> u32 {
        self.doc.len() as u32
    }

    fn push_frame(&mut self, marker: &str, style: Style, hanging: bool) {
        let start = self.markers.len();
        self.markers.push_str(marker);
        self.markers.push(' ');
        let width = marker.width() + 1;
        self.indent += width;
        self.frames.push(Frame {
            start,
            end: self.markers.len(),
            style,
            width,
            hanging,
            started: false,
        });
    }

    fn set_marker(&mut self, marker: &str, style: Style) {
        let Some(f) = self.frames.last_mut() else {
            return;
        };
        self.markers.truncate(f.start);
        self.markers.push_str(marker);
        self.markers.push(' ');
        let width = marker.width() + 1;
        self.indent = self.indent - f.width + width;
        *f = Frame {
            end: self.markers.len(),
            style,
            width,
            ..*f
        };
    }

    fn pop_frame(&mut self) {
        if self.frames.last().is_some_and(|f| !f.started) {
            self.begin_line();
            self.end_line();
        }
        if let Some(f) = self.frames.pop() {
            self.markers.truncate(f.start);
            self.indent -= f.width;
        }
    }

    fn begin_line(&mut self) {
        for f in &mut self.frames {
            if f.hanging && f.started {
                self.doc.pad(f.width, Style::default());
            } else {
                self.doc.push(&self.markers[f.start..f.end], f.style);
            }
            f.started = true;
        }
    }

    /// Blank line that keeps quote bars but not trailing list indentation.
    fn gap(&mut self) {
        let visible = self
            .frames
            .iter()
            .rposition(|f| !f.hanging)
            .map_or(0, |i| i + 1);
        for f in &self.frames[..visible] {
            if f.hanging {
                self.doc.pad(f.width, Style::default());
            } else {
                self.doc.push(&self.markers[f.start..f.end], f.style);
            }
        }
        self.end_line();
    }

    fn end_line(&mut self) {
        self.doc.end_line();
    }
}

/// Parser events with lookahead, needed to size ordered-list numbers up front.
struct Events<'a> {
    parser: OffsetIter<'a>,
    ahead: VecDeque<(Event<'a>, Range<usize>)>,
}

impl<'a> Events<'a> {
    fn new(src: &'a str, options: Options) -> Self {
        Events {
            parser: Parser::new_ext(src, options).into_offset_iter(),
            ahead: VecDeque::new(),
        }
    }

    fn next(&mut self) -> Option<(Event<'a>, Range<usize>)> {
        self.ahead.pop_front().or_else(|| self.parser.next())
    }

    /// Event `i` ahead of the current position.
    fn peek(&mut self, i: usize) -> Option<&Event<'a>> {
        while self.ahead.len() <= i {
            let event = self.parser.next()?;
            self.ahead.push_back(event);
        }
        Some(&self.ahead[i].0)
    }

    /// The image filling the paragraph whose `Start` was just consumed, if nothing but
    /// whitespace or one link around it shares the paragraph.
    fn lone_image(&mut self) -> Option<String> {
        let (mut url, mut depth, mut links) = (None, 0, 0);
        for i in 0.. {
            match self.peek(i)? {
                Event::End(TagEnd::Paragraph) => break,
                Event::Start(Tag::Image { .. }) if depth > 0 || url.is_some() => return None,
                Event::Start(Tag::Image { dest_url, .. }) => {
                    url = Some(dest_url.to_string());
                    depth = 1;
                }
                Event::End(TagEnd::Image) => depth = 0,
                _ if depth > 0 => {}
                Event::Start(Tag::Link { .. }) if links == 0 && url.is_none() => links = 1,
                Event::End(TagEnd::Link) if links == 1 && url.is_some() => links = 2,
                Event::Text(t) if t.trim().is_empty() => {}
                Event::SoftBreak | Event::HardBreak => {}
                _ => return None,
            }
        }
        url.filter(|_| links != 1)
    }

    /// Skips to the end of the current paragraph.
    fn skip_paragraph(&mut self) {
        while let Some((event, _)) = self.next() {
            if matches!(event, Event::End(TagEnd::Paragraph)) {
                break;
            }
        }
    }

    /// Items in the list whose `Start` was just consumed.
    fn count_items(&mut self) -> u64 {
        let (mut depth, mut items) = (0, 0);
        for i in 0.. {
            if i == self.ahead.len() {
                match self.parser.next() {
                    Some(e) => self.ahead.push_back(e),
                    None => break,
                }
            }
            match &self.ahead[i].0 {
                Event::Start(Tag::List(_)) => depth += 1,
                Event::End(TagEnd::List(_)) if depth == 0 => break,
                Event::End(TagEnd::List(_)) => depth -= 1,
                Event::Start(Tag::Item) if depth == 0 => items += 1,
                _ => {}
            }
        }
        items
    }
}

struct List {
    next: Option<u64>,
    digits: usize,
}

struct Table {
    aligns: Vec<Alignment>,
    cells: Vec<(usize, usize)>,  // ranges in the inline scratch
    images: Vec<Option<String>>, // per cell, the image filling it
    rows: Vec<usize>,            // cell count at the end of each row
    head: bool,
    cell_start: usize,
    cell_images: Vec<(String, usize, usize)>, // images of the open cell, with their range
}

struct Builder<'a, 'd> {
    out: Out<'d>,
    theme: &'a Theme,
    inline: Inline,
    styles: Vec<Style>,
    lists: Vec<List>,
    quotes: usize,
    gap: bool,
    at: usize,
    heading: Option<usize>,
    in_code: bool,
    lang: String,
    code: String,
    table: Option<Table>,
    task: Option<usize>,
    link_ids: HashMap<String, u16>,
    heading_text: String,
    slug: String,
    slug_counts: HashMap<String, u32>,
    breaks: Vec<u32>,
    cache: &'d mut Cache,
    pictures: Option<&'d mut dyn Pictures>,
    norm: String, // the open code block with tabs expanded and controls replaced
    num: String,
    natural: Vec<usize>,
    widths: Vec<usize>,
    pieces: Vec<(usize, usize)>,
    cell_pieces: Vec<Range<usize>>,
}

impl<'a, 'd> Builder<'a, 'd> {
    fn new(
        doc: &'d mut Document,
        cache: &'d mut Cache,
        theme: &'a Theme,
        width: usize,
        pictures: Option<&'d mut dyn Pictures>,
    ) -> Self {
        Builder {
            out: Out {
                doc,
                frames: Vec::new(),
                markers: String::new(),
                width,
                indent: 0,
            },
            theme,
            inline: Inline::default(),
            styles: Vec::new(),
            lists: Vec::new(),
            quotes: 0,
            gap: false,
            at: 0,
            heading: None,
            in_code: false,
            lang: String::new(),
            code: String::new(),
            table: None,
            task: None,
            link_ids: HashMap::new(),
            heading_text: String::new(),
            slug: String::new(),
            slug_counts: HashMap::new(),
            breaks: Vec::new(),
            cache,
            pictures,
            norm: String::new(),
            num: String::new(),
            natural: Vec::new(),
            widths: Vec::new(),
            pieces: Vec::new(),
            cell_pieces: Vec::new(),
        }
    }

    fn run(mut self, src: &str) {
        let mut options = OPTIONS;
        options.set(
            Options::ENABLE_MATH,
            cfg!(feature = "math") && self.theme.layout.math,
        );
        let mut events = Events::new(src, options);
        while let Some((event, range)) = events.next() {
            self.at = range.start;
            if self.heading.is_some()
                && let Event::Text(t) | Event::Code(t) = &event
            {
                self.heading_text.push_str(t);
            }
            match event {
                Event::Start(tag) => self.start(tag, &mut events),
                Event::End(tag) => self.end(tag),
                Event::Text(t) if self.in_code => self.code.push_str(&t),
                Event::Text(t) if self.inline.text.is_empty() => {
                    self.inline.push_clean(t.trim_start(), self.cur())
                }
                Event::Text(t) => self.inline.push_clean(&t, self.cur()),
                Event::Code(t) => {
                    let style = self.cur().over(self.theme.styles.code);
                    self.inline.push(" ", style);
                    self.inline.push_clean(&t, style);
                    self.inline.push(" ", style);
                }
                #[cfg(feature = "math")]
                Event::InlineMath(tex) => {
                    let style = self.cur().over(self.theme.styles.math);
                    self.inline.push_clean(&math::inline(&tex), style);
                }
                #[cfg(feature = "math")]
                Event::DisplayMath(tex) => self.display_math(&tex),
                Event::SoftBreak | Event::HardBreak if self.inline.text.is_empty() => {}
                Event::SoftBreak => self.inline.push(" ", self.cur()),
                Event::HardBreak => self.inline.push("\n", self.cur()),
                Event::InlineHtml(h)
                    if h.len() >= 3 && h.as_bytes()[..3].eq_ignore_ascii_case(b"<br") =>
                {
                    self.inline.push("\n", self.cur())
                }
                Event::Rule => self.rule(),
                Event::TaskListMarker(done) => self.task_marker(done, range.start),
                _ => {}
            }
        }
        self.flush_pending();
    }

    fn start(&mut self, tag: Tag, events: &mut Events) {
        let s = &self.theme.styles;
        match tag {
            Tag::Paragraph => {
                self.block();
                if let Some(url) = events.lone_image()
                    && self.picture(&url)
                {
                    events.skip_paragraph();
                    self.gap = true;
                }
            }
            Tag::Heading { level, .. } => {
                self.block();
                self.heading_text.clear();
                self.heading = Some(level as usize);
            }
            Tag::BlockQuote(kind) => self.quote(kind),
            Tag::CodeBlock(kind) => {
                self.block();
                self.in_code = true;
                self.lang.clear();
                if let CodeBlockKind::Fenced(info) = kind {
                    let name = info.split_whitespace().next().unwrap_or("");
                    self.lang.extend(name.chars().filter(|c| !c.is_control()));
                }
            }
            Tag::List(first) => {
                self.block();
                let digits =
                    first.map_or(0, |n| digits(n + events.count_items().saturating_sub(1)));
                self.lists.push(List {
                    next: first,
                    digits,
                });
            }
            Tag::Item => self.item(),
            Tag::Table(aligns) => {
                self.block();
                self.table = Some(Table {
                    aligns,
                    cells: Vec::new(),
                    images: Vec::new(),
                    rows: Vec::new(),
                    head: false,
                    cell_start: 0,
                    cell_images: Vec::new(),
                });
            }
            Tag::TableCell => {
                if let Some(t) = &mut self.table {
                    t.cell_start = self.inline.text.len();
                    t.cell_images.clear();
                }
            }
            Tag::Emphasis => self.push_style(s.emphasis),
            Tag::Strong => self.push_style(s.strong),
            Tag::Strikethrough => self.push_style(s.strike),
            Tag::Link {
                link_type,
                dest_url,
                ..
            } => {
                let id = match link_type {
                    LinkType::Email => self.link_id(&format!("mailto:{dest_url}")),
                    _ => self.link_id(&dest_url),
                };
                self.push_style(Style { link: id, ..s.link });
            }
            Tag::Image { dest_url, .. } => {
                if let Some(t) = &mut self.table {
                    let at = self.inline.text.len();
                    t.cell_images.push((dest_url.to_string(), at, at));
                }
                let id = self.link_id(&dest_url);
                let style = Style {
                    link: id,
                    ..self.cur().over(s.image)
                };
                let icon = &self.theme.glyphs.image;
                if !icon.is_empty() {
                    self.inline.push(icon, style);
                    self.inline.push(" ", style);
                }
                self.styles.push(style);
            }
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => {
                self.paragraph();
                self.gap = true;
            }
            TagEnd::Heading(_) => {
                self.end_heading();
                self.gap = true;
            }
            TagEnd::BlockQuote(kind) => {
                self.flush_pending();
                self.out.pop_frame();
                if kind.is_none() {
                    self.quotes -= 1;
                }
                self.gap = true;
            }
            TagEnd::CodeBlock => {
                self.code_block();
                self.gap = true;
            }
            TagEnd::List(_) => {
                self.flush_pending();
                self.lists.pop();
                self.gap |= self.lists.is_empty();
            }
            TagEnd::Item => {
                self.flush_pending();
                self.finish_task();
                self.out.pop_frame();
            }
            TagEnd::Table => {
                self.table();
                self.gap = true;
            }
            TagEnd::TableHead | TagEnd::TableRow => {
                if let Some(t) = &mut self.table {
                    t.head |= tag == TagEnd::TableHead;
                    t.rows.push(t.cells.len());
                }
            }
            TagEnd::TableCell => {
                if let Some(t) = &mut self.table {
                    let (start, end) = (t.cell_start, self.inline.text.len());
                    let blank = |a: usize, b: usize| self.inline.text[a..b].trim().is_empty();
                    let image = match t.cell_images.as_mut_slice() {
                        [(url, a, b)] if blank(start, *a) && blank(*b, end) => Some(mem::take(url)),
                        _ => None,
                    };
                    t.cells.push((start, end));
                    t.images.push(image);
                }
            }
            TagEnd::Image => {
                self.styles.pop();
                if let Some((_, _, end)) =
                    self.table.as_mut().and_then(|t| t.cell_images.last_mut())
                {
                    *end = self.inline.text.len();
                }
            }
            TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough => {
                self.styles.pop();
            }
            TagEnd::Link => {
                self.styles.pop();
                let style = self.cur().over(self.theme.styles.link_icon);
                self.inline.push(&self.theme.glyphs.link, style);
            }
            _ => {}
        }
    }

    fn cur(&self) -> Style {
        self.styles.last().copied().unwrap_or_default()
    }

    fn push_style(&mut self, style: Style) {
        self.styles.push(self.cur().over(style));
    }

    fn base(&self) -> Style {
        let s = &self.theme.styles;
        if self.quotes > 0 {
            s.text.over(s.quote)
        } else {
            s.text
        }
    }

    fn link_id(&mut self, url: &str) -> u16 {
        if let Some(&id) = self.link_ids.get(url) {
            return id;
        }
        let links = &mut self.out.doc.links;
        let Ok(id) = u16::try_from(links.len() + 1) else {
            return 0;
        };
        links.push(url.chars().filter(|c| !c.is_control()).collect());
        self.link_ids.insert(url.to_owned(), id);
        id
    }

    /// Called at every block start: flushes loose inline text, then the pending blank line.
    fn block(&mut self) {
        self.flush_pending();
        if std::mem::take(&mut self.gap) {
            self.out.gap();
        }
        let line = self.out.line();
        self.out.doc.anchor(line, self.at);
    }

    fn flush_pending(&mut self) {
        if !self.inline.text.is_empty() && self.table.is_none() && self.heading.is_none() {
            self.paragraph();
        }
    }

    fn paragraph(&mut self) {
        self.flush_text();
        self.finish_task();
    }

    /// Flushes inline text as paragraph lines, after the blank line a preceding block asked for.
    fn flush_text(&mut self) {
        if self.inline.text.trim().is_empty() {
            return self.inline.clear();
        }
        if std::mem::take(&mut self.gap) {
            self.out.gap();
        }
        self.flush(self.base(), "", Style::default());
    }

    /// Wraps the inline scratch into lines, with `lead` on the first line and a
    /// hanging indent after it. Returns the first line and the byte offset after `lead`.
    fn flush(&mut self, base: Style, lead: &str, lead_style: Style) -> Option<(u32, u16)> {
        if self.inline.text.is_empty() && lead.is_empty() {
            return None;
        }
        let lead_w = if lead.is_empty() { 0 } else { lead.width() + 1 };
        let text = &self.inline.text;
        wrap::breaks(
            text,
            self.out.avail().saturating_sub(lead_w).max(1),
            &mut self.breaks,
        );
        let mut first = None;
        let mut start = 0;
        for end in self.breaks.iter().map(|&b| b as usize).chain([text.len()]) {
            self.out.begin_line();
            if first.is_none() {
                if lead_w > 0 {
                    self.out.doc.push(lead, lead_style);
                    self.out.doc.push(" ", lead_style);
                }
                first = Some((
                    self.out.line(),
                    self.out.doc.col().min(u16::MAX as usize) as u16,
                ));
            } else {
                self.out.doc.pad(lead_w, Style::default());
            }
            let piece = text[start..end].trim_end_matches([' ', '\n']);
            emit(self.out.doc, &self.inline, start, start + piece.len(), base);
            self.out.end_line();
            start = end;
        }
        self.inline.clear();
        first
    }

    fn end_heading(&mut self) {
        let Some(level) = self.heading.take() else {
            return;
        };
        let t = self.theme;
        let style = t.styles.heading(level);
        let Some((line, title_at)) = self.flush(style, &t.glyphs.heading[level - 1], style) else {
            return;
        };
        let rule = &t.glyphs.heading_rule[level - 1];
        if !rule.is_empty() {
            self.out.begin_line();
            let cols = self.out.avail();
            self.out.doc.fill(rule, cols, style);
            self.out.end_line();
        }
        let headings = &mut self.out.doc.headings;
        let mut parent = headings.len().checked_sub(1);
        while let Some(p) = parent
            && headings[p].level as usize >= level
        {
            parent = headings[p].parent.map(|p| p as usize);
        }
        let parent = parent.map(|p| p as u32);

        self.slug.clear();
        links::slugify(&self.heading_text, &mut self.slug);
        let seen = self.slug_counts.entry(self.slug.clone()).or_insert(0);
        if *seen > 0 {
            let _ = write!(self.slug, "-{seen}");
        }
        *seen += 1;
        self.out
            .doc
            .push_heading(line, level as u8, parent, title_at, &self.slug);
    }

    fn quote(&mut self, kind: Option<BlockQuoteKind>) {
        self.block();
        let t = self.theme;
        let Some(kind) = kind else {
            self.quotes += 1;
            return self
                .out
                .push_frame(&t.glyphs.quote, t.styles.quote_bar, false);
        };
        let (g, s) = (&t.glyphs, &t.styles);
        let (title, style) = match kind {
            BlockQuoteKind::Note => (&g.alert_note, s.alert_note),
            BlockQuoteKind::Tip => (&g.alert_tip, s.alert_tip),
            BlockQuoteKind::Important => (&g.alert_important, s.alert_important),
            BlockQuoteKind::Warning => (&g.alert_warning, s.alert_warning),
            BlockQuoteKind::Caution => (&g.alert_caution, s.alert_caution),
        };
        self.out.push_frame(&g.quote, style, false);
        if !title.is_empty() {
            self.out.begin_line();
            self.out.doc.push(title, style);
            self.out.end_line();
        }
    }

    fn item(&mut self) {
        self.block();
        let t = self.theme;
        let depth = self.lists.len().saturating_sub(1);
        let Some(list) = self.lists.last_mut() else {
            return;
        };
        match list.next {
            Some(n) => {
                self.num.clear();
                let _ = write!(self.num, "{n:>w$}.", w = list.digits);
                list.next = Some(n + 1);
                self.out.push_frame(&self.num, t.styles.number, true);
            }
            None => {
                let bullets = &t.glyphs.bullets;
                self.out
                    .push_frame(&bullets[depth % bullets.len()], t.styles.bullet, true);
            }
        }
    }

    fn task_marker(&mut self, done: bool, offset: usize) {
        let (g, s) = (&self.theme.glyphs, &self.theme.styles);
        let (glyph, style) = if done {
            (&g.task_done, s.task_done)
        } else {
            (&g.task_todo, s.task_todo)
        };
        self.out.set_marker(glyph, style);
        let line = self.out.line();
        self.task = Some(self.out.doc.tasks.len());
        self.out.doc.tasks.push(Task {
            line,
            end: line + 1,
            offset,
            done,
        });
    }

    fn finish_task(&mut self) {
        if let Some(k) = self.task.take() {
            let end = self.out.line();
            let task = &mut self.out.doc.tasks[k];
            task.end = end.max(task.line + 1);
        }
    }

    /// A centered block between the lines of its paragraph; inline in headings and tables,
    /// which can't be split.
    #[cfg(feature = "math")]
    fn display_math(&mut self, tex: &str) {
        let style = self.cur().over(self.theme.styles.math);
        if self.heading.is_some() || self.table.is_some() {
            return self.inline.push_clean(&math::inline(tex), style);
        }
        if !self.inline.text.trim().is_empty() {
            self.flush_text();
            self.gap = true;
        }
        self.inline.clear();
        if std::mem::take(&mut self.gap) {
            self.out.gap();
        }
        self.centered_block(&math::display(tex), style, None);
        self.gap = true;
    }

    /// Leaves room for the image at `url`, to be drawn over these lines; false when it
    /// can't be shown.
    fn picture(&mut self, url: &str) -> bool {
        let avail = self.out.avail();
        let Some(fit) = self.pictures.as_mut().and_then(|p| p.fit(url, avail)) else {
            return false;
        };
        let first = self.out.line();
        for _ in 0..fit.rows {
            self.out.begin_line();
            self.out.doc.pad(fit.cols as usize, Style::default());
            self.out.end_line();
        }
        self.out.doc.pictures.push(Picture {
            first,
            rows: fit.rows,
            col: self.out.indent.min(u16::MAX as usize) as u16,
            cols: fit.cols,
            id: fit.id,
        });
        true
    }

    /// Draws a fenced block through the renderer for its language, instead of a code box.
    /// False when there is none, rendering is off, or the renderer fails.
    fn rendered_block(&mut self) -> bool {
        let theme = self.theme;
        if !theme.layout.diagrams {
            return false;
        }
        let Some(renderer) = theme.renderers.get(&self.lang) else {
            return false;
        };
        let mut code = std::mem::take(&mut self.code);
        let body = code.strip_suffix('\n').unwrap_or(&code);
        let lang = self.lang.to_ascii_lowercase();
        let cx = Context {
            lang: &lang,
            width: self.out.avail(),
            ascii: !theme.layout.icons,
        };
        let lines = self.cache.blocks.render(renderer, body, &cx);
        if let Some(lines) = &lines {
            let first = self.out.line();
            let styles = &theme.styles;
            self.centered_block(lines, styles.diagram, Some(styles.diagram_border));
            self.out.doc.push_code(body, first, self.out.line() - 1);
            code.clear();
            self.in_code = false;
        }
        self.code = code;
        lines.is_some()
    }

    /// Lines centered as one block, or a scrollable wide block when they don't fit.
    /// With `line_style`, diagram lines and arrows get it instead of `style`.
    fn centered_block(&mut self, lines: &[String], style: Style, line_style: Option<Style>) {
        let tab = self.theme.layout.tab_width;
        let lines: Vec<String> = lines
            .iter()
            .map(|line| {
                let mut clean = String::new();
                normalize(line, tab, &mut clean);
                clean
            })
            .collect();
        let avail = self.out.avail();
        let width = lines
            .iter()
            .map(|l| l.width())
            .max()
            .unwrap_or(0)
            .min(MAX_WIDE);
        let wide = width > avail;
        let first = self.out.line();
        for line in &lines {
            let (cut, used) = wrap::fit(line, width);
            self.out.begin_line();
            if wide {
                let start = self.out.doc.col();
                push_styled(self.out.doc, &line[..cut], style, line_style);
                self.out.doc.pad(width - used, Style::default());
                self.out.doc.mark_window(start, self.out.doc.col());
            } else {
                self.out.doc.pad((avail - width) / 2, Style::default());
                push_styled(self.out.doc, &line[..cut], style, line_style);
            }
            self.out.end_line();
        }
        if wide && self.out.line() > first {
            self.out.doc.push_wide(first, avail, width);
        }
    }

    fn rule(&mut self) {
        self.block();
        let t = self.theme;
        self.out.begin_line();
        let cols = self.out.avail();
        self.out.doc.fill(&t.glyphs.rule, cols, t.styles.rule);
        self.out.end_line();
        self.gap = true;
    }

    fn code_block(&mut self) {
        if self.rendered_block() {
            return;
        }
        let Builder {
            out,
            theme,
            code,
            lang,
            norm,
            num,
            cache,
            in_code,
            ..
        } = self;
        *in_code = false;
        let (g, s, layout) = (&theme.glyphs, &theme.styles, &theme.layout);
        let [tl, h, tr, v, bl, br] = &g.code_box;
        let body = code.strip_suffix('\n').unwrap_or(code);
        let avail = out.avail();
        let first = out.line();

        out.begin_line();
        out.doc.push(tl, s.code_border);
        let hw = h.width().max(1);
        let mut rem = avail.saturating_sub(tl.width() + tr.width());
        let lang_w = lang.width() + 2 + hw;
        if !lang.is_empty() && lang_w <= rem {
            out.doc.push(h, s.code_border);
            out.doc.push(" ", s.code_label);
            out.doc.push(lang, s.code_label);
            out.doc.push(" ", s.code_label);
            rem -= lang_w;
        }
        let copy_w = g.code_copy.width() + 2 + hw;
        let copy = !g.code_copy.is_empty() && copy_w <= rem;
        if copy {
            rem -= copy_w;
        }
        out.doc.fill(h, rem, s.code_border);
        if copy {
            out.doc.push(" ", s.code_label);
            out.doc.push(&g.code_copy, s.code_label);
            out.doc.push(" ", s.code_label);
            out.doc.push(h, s.code_border);
        }
        out.doc.push(tr, s.code_border);
        out.end_line();

        let lines = if body.is_empty() {
            0
        } else {
            body.lines().count()
        };
        let num_w = if layout.line_numbers {
            digits(lines as u64) + 2
        } else {
            0
        };
        let inner = avail.saturating_sub(2 * v.width() + 2 + num_w).max(1);
        let numbers = s.code_block.over(s.line_number);
        norm.clear();
        for (i, raw) in body.lines().enumerate() {
            if i > 0 {
                norm.push('\n');
            }
            normalize(raw, layout.tab_width, norm);
        }
        let highlighted = cache.syntax.block(out.doc.code_blocks.len(), lang, norm);
        let natural = norm
            .lines()
            .map(|l| l.width())
            .max()
            .unwrap_or(0)
            .min(MAX_WIDE);
        let wide = natural > inner;
        let span = inner.max(natural);
        let body_first = out.line();
        for (i, line) in norm.lines().enumerate() {
            out.begin_line();
            out.doc.push(v, s.code_border);
            out.doc.push(" ", s.code_block);
            if num_w > 0 {
                num.clear();
                let _ = write!(num, "{:>w$}  ", i + 1, w = num_w - 2);
                out.doc.push(num, numbers);
            }
            let start = out.doc.col();
            let (cut, used) = wrap::fit(line, span);
            let spans = highlighted.map_or(&[][..], |b| b.line(i));
            emit_code(out.doc, &line[..cut], spans, s);
            out.doc.pad(span - used, s.code_block);
            if wide {
                out.doc.mark_window(start, out.doc.col());
            }
            out.doc.push(" ", s.code_block);
            out.doc.push(v, s.code_border);
            out.end_line();
        }
        if wide && out.line() > body_first {
            out.doc.push_wide(body_first, inner, natural);
        }

        out.begin_line();
        out.doc.push(bl, s.code_border);
        out.doc.fill(
            h,
            avail.saturating_sub(bl.width() + br.width()),
            s.code_border,
        );
        out.doc.push(br, s.code_border);
        out.end_line();

        let last = out.line() - 1;
        out.doc.push_code(body, first, last);
        code.clear();
    }

    fn table(&mut self) {
        let base = self.base();
        let Some(tb) = self.table.take() else { return };
        let Builder {
            out,
            theme,
            inline,
            breaks,
            natural,
            widths,
            pieces,
            cell_pieces,
            pictures,
            ..
        } = self;
        let (s, b) = (&theme.styles, &theme.glyphs.table_box);
        let n = tb.aligns.len();
        let budget = out.avail().saturating_sub(3 * n + 1);
        let mut fit = |url: &Option<String>, cols: usize| {
            let url = url.as_deref()?;
            pictures.as_mut()?.fit(url, cols)
        };

        natural.clear();
        natural.resize(n, 0);
        for (row, images) in rows(&tb) {
            for (c, &(a, z)) in row.iter().take(n).enumerate() {
                let w = match fit(&images[c], budget) {
                    Some(f) => f.cols as usize,
                    None => inline.text[a..z]
                        .split('\n')
                        .map(UnicodeWidthStr::width)
                        .max()
                        .unwrap_or(0),
                };
                natural[c] = natural[c].max(w);
            }
        }
        table::column_widths(natural, budget, widths);

        let border = s.table_border;
        let border_w = b[9].width();
        rule_row(out, widths, [&b[0], &b[1], &b[2]], &b[10], border);
        let mut fits = Vec::with_capacity(n);
        for (r, (row, images)) in rows(&tb).enumerate() {
            pieces.clear();
            cell_pieces.clear();
            fits.clear();
            fits.extend(
                widths
                    .iter()
                    .enumerate()
                    .map(|(c, &w)| fit(images.get(c)?, w)),
            );
            for (c, &w) in widths.iter().enumerate() {
                let from = pieces.len();
                if let (None, Some(&(a, z))) = (fits[c], row.get(c)) {
                    wrap::breaks(&inline.text[a..z], w, breaks);
                    let mut start = a;
                    for end in breaks.iter().map(|&x| a + x as usize).chain([z]) {
                        let piece = inline.text[start..end].trim_end_matches([' ', '\n']);
                        pieces.push((start, start + piece.len()));
                        start = end;
                    }
                }
                cell_pieces.push(from..pieces.len());
            }
            let head = tb.head && r == 0;
            let style = if head { s.table_header } else { base };
            let height = cell_pieces
                .iter()
                .map(ExactSizeIterator::len)
                .chain(fits.iter().flatten().map(|f| f.rows as usize))
                .max()
                .unwrap_or(0)
                .max(1);
            let first = out.line();
            for j in 0..height {
                out.begin_line();
                out.doc.push(&b[9], border);
                for (c, &w) in widths.iter().enumerate() {
                    out.doc.push(" ", Style::default());
                    if fits[c].is_some() {
                        out.doc.pad(w + 1, Style::default());
                        out.doc.push(&b[9], border);
                        continue;
                    }
                    match cell_pieces[c].clone().nth(j).map(|k| pieces[k]) {
                        Some((a, z)) => {
                            let slack = w.saturating_sub(inline.text[a..z].width());
                            let (left, right) = table::pad(tb.aligns[c], slack);
                            out.doc.pad(left, Style::default());
                            emit(out.doc, inline, a, z, style);
                            out.doc.pad(right + 1, Style::default());
                        }
                        None => out.doc.pad(w + 1, Style::default()),
                    }
                    out.doc.push(&b[9], border);
                }
                out.end_line();
            }
            let mut col = out.indent + border_w;
            for (c, &w) in widths.iter().enumerate() {
                if let Some(f) = fits[c] {
                    let left = table::pad(tb.aligns[c], w - f.cols as usize).0;
                    out.doc.pictures.push(Picture {
                        first,
                        rows: f.rows,
                        col: (col + 1 + left).min(u16::MAX as usize) as u16,
                        cols: f.cols,
                        id: f.id,
                    });
                }
                col += w + 2 + border_w;
            }
            if head {
                rule_row(out, widths, [&b[3], &b[4], &b[5]], &b[10], border);
            }
        }
        rule_row(out, widths, [&b[6], &b[7], &b[8]], &b[10], border);
        inline.clear();
    }
}

/// Each row's cells and the images filling them.
fn rows(tb: &Table) -> impl Iterator<Item = (&[(usize, usize)], &[Option<String>])> {
    tb.rows.iter().scan(0, |start, &end| {
        let row = (&tb.cells[*start..end], &tb.images[*start..end]);
        *start = end;
        Some(row)
    })
}

fn rule_row(out: &mut Out, widths: &[usize], [left, mid, right]: [&str; 3], h: &str, style: Style) {
    out.begin_line();
    out.doc.push(left, style);
    for (i, &w) in widths.iter().enumerate() {
        out.doc.fill(h, w + 2, style);
        out.doc
            .push(if i + 1 == widths.len() { right } else { mid }, style);
    }
    out.end_line();
}

/// Pushes `text` in `style`, or with diagram lines and arrows in `line_style`.
fn push_styled(doc: &mut Document, text: &str, style: Style, line_style: Option<Style>) {
    let Some(line_style) = line_style else {
        return doc.push(text, style);
    };
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((_, c)) = chars.next() {
        let line = blocks::is_line(c);
        if chars
            .peek()
            .is_none_or(|&(_, n)| blocks::is_line(n) != line)
        {
            let end = chars.peek().map_or(text.len(), |&(j, _)| j);
            doc.push(&text[start..end], if line { line_style } else { style });
            start = end;
        }
    }
}

fn emit_code(doc: &mut Document, line: &str, spans: &[Span], s: &Styles<Style>) {
    let base = s.code_block;
    let mut pos = 0;
    for &(a, b, token) in spans {
        let (a, b) = (a as usize, (b as usize).min(line.len()));
        if a >= line.len() {
            break;
        }
        doc.push(&line[pos..a], base);
        let style = match token {
            Token::Keyword => s.syntax_keyword,
            Token::String => s.syntax_string,
            Token::Number => s.syntax_number,
            Token::Comment => s.syntax_comment,
            Token::Type => s.syntax_type,
            Token::Function => s.syntax_function,
            Token::Constant => s.syntax_constant,
            Token::Operator => s.syntax_operator,
            Token::Tag => s.syntax_tag,
            Token::Attribute => s.syntax_attribute,
            Token::Inserted => s.syntax_inserted,
            Token::Deleted => s.syntax_deleted,
        };
        doc.push(&line[a..b], base.over(style));
        pos = b;
    }
    doc.push(&line[pos..], base);
}

/// Appends `line` with tabs expanded, a trailing `\r` dropped and control characters replaced.
fn normalize(line: &str, tab: usize, out: &mut String) {
    let line = line.strip_suffix('\r').unwrap_or(line);
    if !needs_clean(line) {
        return out.push_str(line);
    }
    let mut col = 0;
    for c in line.chars() {
        match c {
            '\t' => {
                let n = tab - col % tab;
                out.extend(std::iter::repeat_n(' ', n));
                col += n;
            }
            c if c.is_control() => {
                out.push('\u{fffd}');
                col += 1;
            }
            c => {
                out.push(c);
                col += c.width().unwrap_or(0);
            }
        }
    }
}

fn digits(n: u64) -> usize {
    n.checked_ilog10().map_or(1, |d| d as usize + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{BOLD, ITALIC, STRIKE, UNDERLINE, test_theme};

    fn plain(doc: &Document) -> String {
        (0..doc.len())
            .map(|i| doc.line(i).0.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn layout(src: &str, width: usize) -> Document {
        let mut doc = Document::new();
        doc.layout(src, width, &test_theme());
        doc
    }

    fn style_of(doc: &Document, line: usize, needle: &str) -> Style {
        let (text, runs) = doc.line(line);
        let at = text.find(needle).expect("needle on line");
        runs.iter()
            .rev()
            .find(|r| r.at as usize <= at)
            .unwrap()
            .style
    }

    /// Columns a line takes on screen: a wide line shows only `view` of its scrolling part.
    fn visible_width(doc: &Document, i: usize) -> usize {
        let hidden = doc
            .wide_at(i)
            .map_or(0, |k| (doc.wides[k].width - doc.wides[k].view) as usize);
        doc.line(i).0.width() - hidden
    }

    fn check_invariants(doc: &Document) {
        for i in 0..doc.len() {
            let (text, runs) = doc.line(i);
            assert_eq!(text.is_empty(), runs.is_empty(), "line {i} run coverage");
            if let Some(r) = runs.first() {
                assert_eq!(r.at, 0, "line {i} first run");
            }
            for w in runs.windows(2) {
                assert!(w[0].at < w[1].at, "line {i} runs not increasing");
                assert_ne!(w[0].style, w[1].style, "line {i} runs not merged");
                assert!(text.is_char_boundary(w[1].at as usize));
            }
            assert!(!text.contains('\x1b'));
        }
    }

    #[test]
    fn heading_and_paragraph() {
        let doc = layout("# A\n\nhello world", 5);
        assert_eq!(plain(&doc), "██ A\n━━━━━\n\nhello\nworld");
        assert_eq!(doc.title(&doc.headings[0]), "A");
    }

    #[test]
    fn heading_anchors() {
        let doc = layout("# Hello, World!\n## `code` title\n## Hello, World!", 40);
        assert_eq!(doc.find_anchor("hello-world").map(|h| h.line), Some(0));
        assert_eq!(doc.find_anchor("code-title").map(|h| h.level), Some(2));
        assert!(doc.find_anchor("hello-world-1").is_some());
        assert!(doc.find_anchor("nope").is_none());
    }

    #[test]
    fn link_spans_merge_runs() {
        let doc = layout("a [**bold** link](u) and [x](v)", 80);
        let spans: Vec<_> = doc.link_spans(0).collect();
        let text = doc.line(0).0;
        assert_eq!(spans.len(), 2);
        assert_eq!(&text[spans[0].0..spans[0].1], "bold link");
        assert_eq!(&text[spans[1].0..spans[1].1], "x");
        assert_eq!(doc.links[spans[1].2 as usize - 1], "v");
    }

    #[test]
    fn wrapped_link_continues() {
        let doc = layout("[aaa bbb](u)", 5);
        let id = doc.link_spans(1).next().unwrap().2;
        assert!(doc.continues_link(1, id));
        assert!(!doc.continues_link(0, id));
    }

    #[test]
    fn heading_parents() {
        let doc = layout("# A\n## B\n### C\n## D\n# E", 40);
        let parents: Vec<_> = doc.headings.iter().map(|h| h.parent).collect();
        assert_eq!(parents, [None, Some(0), Some(1), Some(0), None]);
    }

    #[test]
    fn inline_styles() {
        let doc = layout("**b** *i* ~~s~~ ***bi*** `c` [l](http://x)", 80);
        let t = test_theme();
        assert!(style_of(&doc, 0, "b ").attrs & BOLD != 0);
        assert!(style_of(&doc, 0, "i ").attrs & ITALIC != 0);
        assert!(style_of(&doc, 0, "s ").attrs & STRIKE != 0);
        assert_eq!(
            style_of(&doc, 0, "bi").attrs & (BOLD | ITALIC),
            BOLD | ITALIC
        );
        assert_eq!(style_of(&doc, 0, " c ").bg, t.styles.code.bg);
        let link = style_of(&doc, 0, "l↗");
        assert!(link.attrs & UNDERLINE != 0);
        assert_eq!(doc.links[link.link as usize - 1], "http://x");
        assert_eq!(style_of(&doc, 0, "↗").link, 0);
        assert!(plain(&doc).ends_with(" c  l↗"));
    }

    #[test]
    fn links_are_deduplicated() {
        let doc = layout("[a](u) [b](u) [c](v)", 80);
        assert_eq!(doc.links, ["u", "v"]);
    }

    #[test]
    fn hard_break_and_wrap_carry_style() {
        let doc = layout("**one two**\\\nthree", 4);
        assert_eq!(plain(&doc), "one\ntwo\nthre\ne");
        assert!(style_of(&doc, 1, "two").attrs & BOLD != 0);
        check_invariants(&doc);
    }

    #[test]
    fn nested_lists() {
        let doc = layout("- a\n  - b\n    - c\n- d", 40);
        assert_eq!(plain(&doc), "• a\n  ◦ b\n    ▪ c\n• d");
    }

    #[test]
    fn ordered_numbers_right_aligned() {
        let src: String = (1..=10).map(|i| format!("{i}. x\n")).collect();
        let doc = layout(&src, 40);
        assert_eq!(doc.line(0).0, " 1. x");
        assert_eq!(doc.line(9).0, "10. x");
    }

    #[test]
    fn hanging_indent() {
        let doc = layout("- aaa bbb", 6);
        assert_eq!(plain(&doc), "• aaa\n  bbb");
    }

    #[test]
    fn frames_nest_without_special_cases() {
        let doc = layout("> - a\n>   > b\n>   > c", 40);
        assert_eq!(plain(&doc), "▌ • a\n▌   ▌ b c");
    }

    #[test]
    fn loose_list_gap_keeps_quote_bar() {
        let doc = layout("> one\n>\n> two", 40);
        assert_eq!(plain(&doc), "▌ one\n▌\n▌ two");
    }

    /// Images named `*.png` are 3 rows tall and at most 10 columns wide.
    struct Fixed;

    impl Pictures for Fixed {
        fn fit(&mut self, url: &str, cols: usize) -> Option<Fit> {
            url.ends_with(".png").then_some(Fit {
                id: 7,
                cols: cols.min(10) as u16,
                rows: 3,
            })
        }
    }

    fn with_pictures(src: &str) -> Document {
        let mut doc = Document::new();
        doc.layout_with(src, 40, &test_theme(), Some(&mut Fixed));
        doc
    }

    #[test]
    fn lone_images_become_pictures() {
        let doc = with_pictures("before\n\n![logo](a.png)\n\nafter");
        assert_eq!(plain(&doc), "before\n\n\n\n\n\nafter");
        let picture = Picture {
            first: 2,
            rows: 3,
            col: 0,
            cols: 10,
            id: 7,
        };
        assert_eq!(doc.pictures, [picture]);
        assert_eq!(doc.pictures_at(4).collect::<Vec<_>>(), [&picture]);
        assert_eq!(doc.pictures_at(5).count(), 0);
        assert_eq!(
            with_pictures("[![logo](a.png)](https://x)").pictures.len(),
            1
        );
    }

    #[test]
    fn pictures_keep_their_indent() {
        let doc = with_pictures("> ![logo](a.png)");
        assert_eq!(doc.pictures[0].col, 2);
        assert_eq!(plain(&doc).lines().next(), Some("▌"));
    }

    #[test]
    fn table_cells_hold_pictures_sized_to_their_column() {
        let doc = with_pictures(
            "| a | b |\n|---|---|\n| ![x](a.png) | ![y](b.png) |\n| c | ![z](z.gif) |",
        );
        let cells: Vec<_> = doc
            .pictures
            .iter()
            .map(|p| (p.first, p.col, p.cols, p.rows))
            .collect();
        assert_eq!(cells, [(3, 2, 10, 3), (3, 15, 10, 3)]);
        assert_eq!(doc.pictures_at(5).count(), 2);
        let text = plain(&doc);
        let lines: Vec<_> = text.lines().collect();
        assert_eq!(lines[3], lines[5]);
        assert_eq!(lines[3].trim_end(), "│            │            │");
        assert!(lines[6].contains("🖼"));
    }

    #[test]
    fn images_sharing_a_paragraph_stay_text() {
        for src in [
            "see ![logo](a.png)",
            "![a](a.png) ![b](b.png)",
            "[![a](a.png)](u) [x](v)",
            "![missing](a.gif)",
            "# ![logo](a.png)",
            "| ![a](a.png) and text |\n|---|",
        ] {
            let doc = with_pictures(src);
            assert!(doc.pictures.is_empty(), "{src}");
            assert!(plain(&doc).contains('🖼'), "{src}");
        }
    }

    #[test]
    fn alerts() {
        let doc = layout("> [!WARNING]\n> Careful.", 40);
        let t = test_theme();
        assert!(plain(&doc).ends_with("WARNING\n▌ Careful."));
        assert_eq!(style_of(&doc, 0, "WARNING").fg, t.styles.alert_warning.fg);
    }

    #[cfg(feature = "math")]
    #[test]
    fn inline_math() {
        let doc = layout("Area $\\pi r^2$ here", 80);
        assert_eq!(plain(&doc), "Area π r² here");
        assert_eq!(style_of(&doc, 0, "π r²").fg, test_theme().styles.math.fg);
    }

    #[cfg(feature = "math")]
    #[test]
    fn display_math_is_a_centered_block() {
        let doc = layout("before\n$$x^2$$\nafter", 20);
        assert_eq!(plain(&doc), "before\n\n         x²\n\nafter");
        let doc = layout("$$\\begin{pmatrix} a & b \\\\ c & d \\end{pmatrix}$$", 13);
        assert_eq!(plain(&doc), "  ⎛ a   b ⎞\n  ⎝ c   d ⎠");
    }

    #[cfg(feature = "math")]
    #[test]
    fn multi_line_math_is_centered_as_a_block() {
        let doc = layout(
            r"$$f(x) = \begin{cases} 1 & x > 0 \\ 0 & \text{otherwise} \end{cases}$$",
            40,
        );
        let (a, b) = (doc.line(0).0, doc.line(1).0);
        assert_eq!(
            a.find('⎧').map(|i| a[..i].width()),
            b.find('⎩').map(|i| b[..i].width())
        );
    }

    #[cfg(feature = "math")]
    #[test]
    fn text_after_display_math_has_no_leading_space() {
        let doc = layout("- item $$x^2$$ after", 40);
        assert!(plain(&doc).ends_with("\n  after"));
    }

    #[cfg(feature = "math")]
    #[test]
    fn display_math_stays_inline_in_tables() {
        let doc = layout("| a |\n|---|\n| $$x^2$$ |", 20);
        assert!(plain(&doc).contains("│ x² │"));
    }

    #[cfg(not(feature = "math"))]
    #[test]
    fn math_without_the_feature_stays_text() {
        assert_eq!(plain(&layout("$x^2$ and $$y$$", 80)), "$x^2$ and $$y$$");
    }

    #[cfg(unix)]
    #[test]
    fn renderer_output_tabs_are_expanded() {
        let mut cfg = crate::config::defaults();
        cfg.renderers.insert(
            "tabs".into(),
            crate::config::Command::Args(vec!["printf".into(), "a\tb\tEND".into()]),
        );
        let mut doc = Document::new();
        doc.layout("```tabs\nx\n```", 40, &Theme::new(cfg).unwrap());
        assert!(plain(&doc).contains("END"));
    }

    #[test]
    fn dollar_amounts_are_not_math() {
        let doc = layout("costs $5 and $10", 80);
        assert_eq!(plain(&doc), "costs $5 and $10");
    }

    #[test]
    fn math_can_be_turned_off() {
        let mut cfg = crate::config::defaults();
        cfg.layout.math = false;
        let mut doc = Document::new();
        doc.layout("$x^2$ and $$y$$", 80, &Theme::new(cfg).unwrap());
        assert_eq!(plain(&doc), "$x^2$ and $$y$$");
    }

    #[cfg(feature = "mermaid")]
    #[test]
    fn mermaid_blocks_become_diagrams() {
        let src = "```mermaid\ngraph LR\n  A[Start] --> B[End]\n```";
        let doc = layout(src, 60);
        let text = plain(&doc);
        assert!(text.contains("Start") && text.contains("End") && !text.contains("graph LR"));
        assert_eq!(doc.code(0), "graph LR\n  A[Start] --> B[End]");
        let row = (0..doc.len())
            .find(|&i| doc.line(i).0.contains("Start"))
            .unwrap();
        let t = test_theme();
        assert_eq!(style_of(&doc, row, "Start").fg, t.styles.diagram.fg);
        assert_eq!(style_of(&doc, row, "│").fg, t.styles.diagram_border.fg);
    }

    #[cfg(unix)]
    #[test]
    fn configured_renderers_draw_blocks() {
        let theme = |diagrams: bool, command: &str| {
            let mut cfg = crate::config::defaults();
            cfg.layout.diagrams = diagrams;
            cfg.renderers
                .insert("shout".into(), crate::config::Command::Line(command.into()));
            Theme::new(cfg).unwrap()
        };
        let src = "```shout\nhello\n```";
        let mut doc = Document::new();
        doc.layout(src, 40, &theme(true, "tr a-z A-Z"));
        assert!(plain(&doc).contains("HELLO") && !plain(&doc).contains("hello"));
        assert_eq!(doc.code(0), "hello");
        doc.layout(src, 40, &theme(false, "tr a-z A-Z"));
        assert!(plain(&doc).contains("│ 1  hello"));
        doc.layout(src, 40, &theme(true, "false"));
        assert!(plain(&doc).contains("│ 1  hello"));
    }

    #[test]
    fn broken_diagrams_show_their_source() {
        let doc = layout("```mermaid\nnot a diagram\n```", 40);
        assert!(plain(&doc).contains("│ 1  not a diagram"));
        let mut cfg = crate::config::defaults();
        cfg.layout.diagrams = false;
        let mut off = Document::new();
        off.layout(
            "```mermaid\ngraph LR\n  A --> B\n```",
            40,
            &Theme::new(cfg).unwrap(),
        );
        assert!(plain(&off).contains("graph LR"));
    }

    #[cfg(feature = "mermaid")]
    #[test]
    fn wide_diagrams_scroll() {
        let src = "```mermaid\ngraph LR\n  A[First step] --> B[Second step] --> C[Third step] --> D[Fourth step]\n```";
        let doc = layout(src, 30);
        let row = (0..doc.len())
            .find(|&i| doc.line(i).0.contains("First"))
            .unwrap();
        let k = doc
            .wide_at(row)
            .expect("diagram wider than 30 columns scrolls");
        assert!(doc.wides[k].width > doc.wides[k].view);
    }

    #[test]
    fn tasks_record_source_offsets() {
        let src = "- [ ] a\n- [x] b\n";
        let doc = layout(src, 40);
        assert_eq!(plain(&doc), "☐ a\n✔ b");
        assert_eq!(doc.tasks.len(), 2);
        assert_eq!(&src[doc.tasks[0].offset..][..3], "[ ]");
        assert_eq!(&src[doc.tasks[1].offset..][..3], "[x]");
        assert!(doc.tasks[1].done);
        assert_eq!(doc.task_at(1).map(|t| t.line), Some(1));
    }

    #[test]
    fn wrapped_task_covers_all_lines() {
        let doc = layout("- [ ] aaa bbb ccc\n- x", 8);
        assert_eq!(plain(&doc), "☐ aaa\n  bbb\n  ccc\n• x");
        assert!(doc.task_at(2).is_some());
        assert!(doc.task_at(3).is_none());
    }

    #[test]
    fn code_block_box() {
        let doc = layout("```rust\nfn main() {}\n```", 30);
        let text = plain(&doc);
        let lines: Vec<_> = text.lines().collect();
        assert_eq!(lines[0], "╭─ rust ────────── [y] copy ─╮");
        assert_eq!(lines[1], "│ 1  fn main() {}            │");
        assert_eq!(lines[2], "╰────────────────────────────╯");
        assert!(lines.iter().all(|l| l.width() == 30));
        assert_eq!(doc.code(0), "fn main() {}");
        assert_eq!((doc.code_blocks[0].first, doc.code_blocks[0].last), (0, 2));
    }

    #[test]
    fn long_code_lines_become_a_wide_block() {
        let doc = layout("```\nabcdefghijklmnopqrstuvwxyz\nshort\n```", 20);
        assert_eq!(doc.line(1).0, "│ 1  abcdefghijklmnopqrstuvwxyz │");
        assert_eq!(doc.line(2).0, "│ 2  short                      │");
        let k = doc.wide_at(1).unwrap();
        assert_eq!(doc.wide_at(2), Some(k));
        assert_eq!((doc.wide_at(0), doc.wide_at(3)), (None, None));
        assert_eq!((doc.wides[k].view, doc.wides[k].width), (13, 26));
        let (a, b) = doc.window(k, 1);
        assert_eq!(&doc.line(1).0[a..b], "abcdefghijklmnopqrstuvwxyz");
        assert_eq!(visible_width(&doc, 1), 20);
    }

    #[test]
    fn fitting_code_is_not_wide() {
        let doc = layout("```\nshort\n```", 20);
        assert!(doc.wides.is_empty());
    }

    #[cfg(feature = "math")]
    #[test]
    fn wide_display_math_scrolls() {
        let doc = layout("$$a + b + c + d + e + f + g + h + i + j$$", 12);
        let k = doc.wide_at(0).unwrap();
        assert_eq!(doc.wides[k].view, 12);
        assert_eq!(doc.window(k, 0), (0, doc.line(0).0.len()));
    }

    #[test]
    fn tables() {
        let doc = layout("| a | bb |\n|:-:|---:|\n| ccc | d |", 40);
        assert_eq!(
            plain(&doc),
            "┌─────┬────┐\n│  a  │ bb │\n├─────┼────┤\n│ ccc │  d │\n└─────┴────┘"
        );
        let doc = layout("| a | b |\n|---|---|\n| one two three | x |", 16);
        assert!((0..doc.len()).all(|i| doc.line(i).0.width() <= 16));
        check_invariants(&doc);
    }

    #[test]
    fn rule_and_gaps() {
        let doc = layout("a\n\n---\n\nb", 3);
        assert_eq!(plain(&doc), "a\n\n───\n\nb");
    }

    #[test]
    fn control_characters_are_replaced() {
        let doc = layout("a\x1b[31mb", 40);
        assert_eq!(plain(&doc), "a\u{fffd}[31mb");
    }

    #[test]
    fn anchors_map_lines_to_source() {
        let src = "# A\n\npara one\n\npara two\n";
        let doc = layout(src, 40);
        let line = (0..doc.len())
            .find(|&i| doc.line(i).0 == "para two")
            .unwrap();
        assert_eq!(&src[doc.source_offset(line)..][..8], "para two");
        assert_eq!(doc.line_at_offset(src.find("para two").unwrap()), line);
    }

    #[test]
    fn docs_invariants() {
        let docs = [
            include_str!("../README.md"),
            include_str!("../docs/configuration.md"),
            include_str!("../docs/demo.md"),
        ];
        for src in docs {
            for width in [20, 60, 90] {
                let doc = layout(src, width);
                check_invariants(&doc);
                assert!((0..doc.len()).all(|i| visible_width(&doc, i) <= width.max(30)));
            }
        }
    }
}
