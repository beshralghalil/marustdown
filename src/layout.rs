use std::collections::{HashMap, VecDeque};
use std::fmt::Write as _;
use std::ops::Range;

use pulldown_cmark::{
    Alignment, BlockQuoteKind, CodeBlockKind, Event, LinkType, OffsetIter, Options, Parser, Tag,
    TagEnd,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::config::Styles;
use crate::doc::{Document, Heading, Task};
use crate::highlight::{Highlighter, Span, Token};
use crate::theme::{Style, Theme};
use crate::{table, wrap};

const OPTIONS: Options = Options::ENABLE_TABLES
    .union(Options::ENABLE_STRIKETHROUGH)
    .union(Options::ENABLE_TASKLISTS)
    .union(Options::ENABLE_GFM);

/// Lays `src` out into `doc` at `width` columns.
pub fn build(doc: &mut Document, src: &str, width: usize, theme: &Theme) {
    Builder::new(doc, theme, width).run(src);
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
    fn new(src: &'a str) -> Self {
        Events {
            parser: Parser::new_ext(src, OPTIONS).into_offset_iter(),
            ahead: VecDeque::new(),
        }
    }

    fn next(&mut self) -> Option<(Event<'a>, Range<usize>)> {
        self.ahead.pop_front().or_else(|| self.parser.next())
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
    cells: Vec<(usize, usize)>, // ranges in the inline scratch
    rows: Vec<usize>,           // cell count at the end of each row
    head: bool,
    cell_start: usize,
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
    breaks: Vec<u32>,
    spans: Vec<Span>,
    line_buf: String,
    num: String,
    natural: Vec<usize>,
    widths: Vec<usize>,
    pieces: Vec<(usize, usize)>,
    cell_pieces: Vec<Range<usize>>,
}

impl<'a, 'd> Builder<'a, 'd> {
    fn new(doc: &'d mut Document, theme: &'a Theme, width: usize) -> Self {
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
            breaks: Vec::new(),
            spans: Vec::new(),
            line_buf: String::new(),
            num: String::new(),
            natural: Vec::new(),
            widths: Vec::new(),
            pieces: Vec::new(),
            cell_pieces: Vec::new(),
        }
    }

    fn run(mut self, src: &str) {
        let mut events = Events::new(src);
        while let Some((event, range)) = events.next() {
            self.at = range.start;
            match event {
                Event::Start(tag) => self.start(tag, &mut events),
                Event::End(tag) => self.end(tag),
                Event::Text(t) if self.in_code => self.code.push_str(&t),
                Event::Text(t) => self.inline.push_clean(&t, self.cur()),
                Event::Code(t) => {
                    let style = self.cur().over(self.theme.styles.code);
                    self.inline.push(" ", style);
                    self.inline.push_clean(&t, style);
                    self.inline.push(" ", style);
                }
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
            Tag::Paragraph => self.block(),
            Tag::Heading { level, .. } => {
                self.block();
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
                    rows: Vec::new(),
                    head: false,
                    cell_start: 0,
                });
            }
            Tag::TableCell => {
                if let Some(t) = &mut self.table {
                    t.cell_start = self.inline.text.len();
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
                    t.cells.push((t.cell_start, self.inline.text.len()));
                }
            }
            TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough | TagEnd::Image => {
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
        self.flush(self.base(), "", Style::default());
        self.finish_task();
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
        headings.push(Heading {
            line,
            level: level as u8,
            parent: parent.map(|p| p as u32),
            title_at,
        });
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
        let Builder {
            out,
            theme,
            code,
            lang,
            line_buf,
            num,
            spans,
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
        let ellipsis_w = g.ellipsis.width();
        let mut highlighter = Highlighter::new(lang);
        let numbers = s.code_block.over(s.line_number);
        for (i, raw) in body.lines().enumerate() {
            normalize(raw, layout.tab_width, line_buf);
            out.begin_line();
            out.doc.push(v, s.code_border);
            out.doc.push(" ", s.code_block);
            if num_w > 0 {
                num.clear();
                let _ = write!(num, "{:>w$}  ", i + 1, w = num_w - 2);
                out.doc.push(num, numbers);
            }
            let (mut cut, mut used) = wrap::fit(line_buf, inner);
            let truncated = cut < line_buf.len();
            if truncated {
                (cut, used) = wrap::fit(line_buf, inner.saturating_sub(ellipsis_w));
            }
            match &mut highlighter {
                Some(hl) => {
                    hl.line(line_buf, spans);
                    emit_code(out.doc, &line_buf[..cut], spans, s);
                }
                None => out.doc.push(&line_buf[..cut], s.code_block),
            }
            if truncated {
                out.doc.push(&g.ellipsis, numbers);
                used += ellipsis_w;
            }
            out.doc.pad(inner.saturating_sub(used) + 1, s.code_block);
            out.doc.push(v, s.code_border);
            out.end_line();
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
            ..
        } = self;
        let (s, b) = (&theme.styles, &theme.glyphs.table_box);
        let n = tb.aligns.len();

        natural.clear();
        natural.resize(n, 0);
        for row in rows(&tb) {
            for (c, &(a, z)) in row.iter().take(n).enumerate() {
                let w = inline.text[a..z]
                    .split('\n')
                    .map(UnicodeWidthStr::width)
                    .max()
                    .unwrap_or(0);
                natural[c] = natural[c].max(w);
            }
        }
        table::column_widths(natural, out.avail().saturating_sub(3 * n + 1), widths);

        let border = s.table_border;
        rule_row(out, widths, [&b[0], &b[1], &b[2]], &b[10], border);
        for (r, row) in rows(&tb).enumerate() {
            pieces.clear();
            cell_pieces.clear();
            for (c, &w) in widths.iter().enumerate() {
                let from = pieces.len();
                if let Some(&(a, z)) = row.get(c) {
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
                .max()
                .unwrap_or(0)
                .max(1);
            for j in 0..height {
                out.begin_line();
                out.doc.push(&b[9], border);
                for (c, &w) in widths.iter().enumerate() {
                    out.doc.push(" ", Style::default());
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
            if head {
                rule_row(out, widths, [&b[3], &b[4], &b[5]], &b[10], border);
            }
        }
        rule_row(out, widths, [&b[6], &b[7], &b[8]], &b[10], border);
        inline.clear();
    }
}

fn rows(tb: &Table) -> impl Iterator<Item = &[(usize, usize)]> {
    tb.rows.iter().scan(0, |start, &end| {
        let row = &tb.cells[*start..end];
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
        };
        doc.push(&line[a..b], base.over(style));
        pos = b;
    }
    doc.push(&line[pos..], base);
}

/// Expands tabs, drops a trailing `\r` and replaces control characters.
fn normalize(line: &str, tab: usize, out: &mut String) {
    out.clear();
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

    #[test]
    fn alerts() {
        let doc = layout("> [!WARNING]\n> Careful.", 40);
        let t = test_theme();
        assert!(plain(&doc).ends_with("WARNING\n▌ Careful."));
        assert_eq!(style_of(&doc, 0, "WARNING").fg, t.styles.alert_warning.fg);
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
    fn long_code_lines_truncate() {
        let doc = layout("```\nabcdefghijklmnopqrstuvwxyz\n```", 20);
        assert_eq!(doc.line(1).0, "│ 1  abcdefghijkl… │");
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
    fn sample_document_invariants() {
        let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/data/sample.md"))
            .unwrap();
        for width in [20, 60, 90] {
            let doc = layout(&src, width);
            check_invariants(&doc);
            assert!((0..doc.len()).all(|i| doc.line(i).0.width() <= width.max(30)));
        }
    }
}
