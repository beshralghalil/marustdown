use unicode_width::UnicodeWidthStr;

use crate::layout;
use crate::theme::{Style, Theme};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Run {
    pub at: u16, // relative to line start
    pub style: Style,
}

#[derive(Clone, Copy)]
struct LineMeta {
    text_end: u32,
    runs_end: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Heading {
    pub line: u32,
    pub level: u8,
    pub parent: Option<u32>,
    title_at: u16,
    slug_end: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct CodeBlock {
    pub first: u32,
    pub last: u32,
    end: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Task {
    pub line: u32,
    pub end: u32,      // first line after the item's text
    pub offset: usize, // source offset of the `[`
    pub done: bool,
}

/// Lines `first..=last` whose scrolling part can be wider than the screen. Only `view`
/// of its `width` columns are shown at a time, from a horizontal offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wide {
    pub first: u32,
    pub last: u32,
    pub view: u16,
    pub width: u16,
    windows: u32, // index of the first line's window
}

/// Lines `first..first + rows` where an image is drawn over columns `col..col + cols`,
/// counted from the content's left edge. `id` names the image for whoever draws it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Picture {
    pub first: u32,
    pub rows: u16,
    pub col: u16,
    pub cols: u16,
    pub id: u32,
}

/// Laid-out text in three flat buffers: plain text, style runs, and line ends.
#[derive(Default)]
pub struct Document {
    text: String,
    runs: Vec<Run>,
    lines: Vec<LineMeta>,
    pub links: Vec<String>,
    pub headings: Vec<Heading>,
    pub code_blocks: Vec<CodeBlock>,
    pub tasks: Vec<Task>,
    pub wides: Vec<Wide>,
    pub pictures: Vec<Picture>,
    windows: Vec<(u16, u16)>, // byte range of each wide line's scrolling part
    anchors: Vec<(u32, usize)>, // (line, source offset) at each block start
    code: String,
    slugs: String,
    cache: layout::Cache, // survives `clear`, so re-layouts reuse unchanged blocks
}

impl Document {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn line(&self, i: usize) -> (&str, &[Run]) {
        let (t0, r0) = self.line_start(i);
        let m = &self.lines[i];
        (
            &self.text[t0..m.text_end as usize],
            &self.runs[r0..m.runs_end as usize],
        )
    }

    pub fn title(&self, h: &Heading) -> &str {
        &self.line(h.line as usize).0[h.title_at as usize..]
    }

    /// The heading a `#fragment` link points to.
    pub fn find_anchor(&self, fragment: &str) -> Option<&Heading> {
        let mut start = 0;
        self.headings.iter().find(|h| {
            let slug = &self.slugs[start..h.slug_end as usize];
            start = h.slug_end as usize;
            slug.eq_ignore_ascii_case(fragment)
        })
    }

    /// Link spans on line `i` as (start, end, link id), merging adjacent runs of one link.
    pub fn link_spans(&self, i: usize) -> impl Iterator<Item = (usize, usize, u16)> + '_ {
        let (text, runs) = self.line(i);
        let mut k = 0;
        std::iter::from_fn(move || {
            k += runs[k..].iter().position(|r| r.style.link != 0)?;
            let (start, id) = (runs[k].at as usize, runs[k].style.link);
            k += runs[k..]
                .iter()
                .position(|r| r.style.link != id)
                .unwrap_or(runs.len() - k);
            let end = runs.get(k).map_or(text.len(), |r| r.at as usize);
            Some((start, end, id))
        })
    }

    /// True if link `id` on line `i` is the wrapped tail of a link from the line above.
    pub fn continues_link(&self, i: usize, id: u16) -> bool {
        i > 0
            && self
                .line(i - 1)
                .1
                .last()
                .is_some_and(|r| r.style.link == id)
    }

    pub fn code(&self, k: usize) -> &str {
        let start = k
            .checked_sub(1)
            .map_or(0, |p| self.code_blocks[p].end as usize);
        &self.code[start..self.code_blocks[k].end as usize]
    }

    pub fn task_at(&self, line: usize) -> Option<&Task> {
        let k = self
            .tasks
            .partition_point(|t| t.line as usize <= line)
            .checked_sub(1)?;
        Some(&self.tasks[k]).filter(|t| line < t.end as usize)
    }

    /// Index of the wide block containing `line`.
    pub fn wide_at(&self, line: usize) -> Option<usize> {
        let k = self.wides.partition_point(|w| (w.last as usize) < line);
        self.wides
            .get(k)
            .filter(|w| w.first as usize <= line)
            .map(|_| k)
    }

    /// Pictures drawn over `line`. Those sharing lines, as in a table row, start together.
    pub fn pictures_at(&self, line: usize) -> impl Iterator<Item = &Picture> {
        let end = self.pictures.partition_point(|p| p.first as usize <= line);
        let first = end.checked_sub(1).map(|k| self.pictures[k].first);
        self.pictures[..end]
            .iter()
            .rev()
            .take_while(move |p| Some(p.first) == first)
            .filter(move |p| line < p.first as usize + p.rows as usize)
    }

    /// Byte range of the scrolling part of `line`, which belongs to wide block `k`.
    pub fn window(&self, k: usize, line: usize) -> (usize, usize) {
        let w = &self.wides[k];
        let (start, end) = self.windows[w.windows as usize + line - w.first as usize];
        (start as usize, end as usize)
    }

    pub fn source_offset(&self, line: usize) -> usize {
        let k = self.anchors.partition_point(|a| a.0 as usize <= line);
        k.checked_sub(1).map_or(0, |k| self.anchors[k].1)
    }

    /// Source byte range of the block `line` belongs to; `None` end means to the end.
    pub fn source_range(&self, line: usize) -> (usize, Option<usize>) {
        let k = self.anchors.partition_point(|a| a.0 as usize <= line);
        match k.checked_sub(1) {
            Some(k) => (self.anchors[k].1, self.anchors.get(k + 1).map(|a| a.1)),
            None => (0, self.anchors.first().map(|a| a.1)),
        }
    }

    pub fn line_at_offset(&self, offset: usize) -> usize {
        let k = self.anchors.partition_point(|a| a.1 <= offset);
        k.checked_sub(1).map_or(0, |k| self.anchors[k].0 as usize)
    }

    /// Empties every buffer but keeps its capacity, so a re-layout allocates nothing.
    pub fn clear(&mut self) {
        self.text.clear();
        self.runs.clear();
        self.lines.clear();
        self.links.clear();
        self.headings.clear();
        self.code_blocks.clear();
        self.tasks.clear();
        self.wides.clear();
        self.pictures.clear();
        self.windows.clear();
        self.anchors.clear();
        self.code.clear();
        self.slugs.clear();
    }

    pub fn layout(&mut self, src: &str, width: usize, theme: &Theme) {
        self.layout_with(src, width, theme, None);
    }

    /// Lays out with images drawn as pictures where `pictures` can size them.
    pub fn layout_with(
        &mut self,
        src: &str,
        width: usize,
        theme: &Theme,
        pictures: Option<&mut dyn layout::Pictures>,
    ) {
        self.clear();
        let mut cache = std::mem::take(&mut self.cache);
        layout::build(self, &mut cache, src, width, theme, pictures);
        self.cache = cache;
    }

    /// Records that `line` starts the block at source byte `offset`.
    pub(crate) fn anchor(&mut self, line: u32, offset: usize) {
        match self.anchors.last_mut() {
            Some(a) if a.0 == line => a.1 = offset,
            _ => self.anchors.push((line, offset)),
        }
    }

    pub(crate) fn push_heading(
        &mut self,
        line: u32,
        level: u8,
        parent: Option<u32>,
        title_at: u16,
        slug: &str,
    ) {
        self.slugs.push_str(slug);
        let slug_end = self.slugs.len() as u32;
        self.headings.push(Heading {
            line,
            level,
            parent,
            title_at,
            slug_end,
        });
    }

    /// Marks bytes `start..end` of the open line as its scrolling part.
    pub(crate) fn mark_window(&mut self, start: usize, end: usize) {
        let offset = |n: usize| u16::try_from(n).expect("wide line under 64 KiB");
        self.windows.push((offset(start), offset(end)));
    }

    /// Closes a wide block over the lines marked since `first`.
    pub(crate) fn push_wide(&mut self, first: u32, view: usize, width: usize) {
        let last = self.lines.len() as u32 - 1;
        let windows = (self.windows.len() - (last - first + 1) as usize) as u32;
        let cols = |n: usize| n.min(u16::MAX as usize) as u16;
        self.wides.push(Wide {
            first,
            last,
            view: cols(view),
            width: cols(width),
            windows,
        });
    }

    pub(crate) fn push_code(&mut self, code: &str, first: u32, last: u32) {
        self.code.push_str(code);
        let end = self.code.len() as u32;
        self.code_blocks.push(CodeBlock { first, last, end });
    }

    fn line_start(&self, i: usize) -> (usize, usize) {
        i.checked_sub(1).map_or((0, 0), |p| {
            let m = &self.lines[p];
            (m.text_end as usize, m.runs_end as usize)
        })
    }

    pub(crate) fn col(&self) -> usize {
        self.text.len() - self.line_start(self.lines.len()).0
    }

    pub(crate) fn push(&mut self, s: &str, style: Style) {
        if s.is_empty() {
            return;
        }
        let (line_start, run_start) = self.line_start(self.lines.len());
        let at = self.text.len() - line_start;
        let fresh = self.runs.len() == run_start;
        // Past u16 range the style is frozen instead of overflowing `at`.
        if (fresh || self.runs.last().is_some_and(|r| r.style != style)) && at <= u16::MAX as usize
        {
            self.runs.push(Run {
                at: at as u16,
                style,
            });
        }
        self.text.push_str(s);
    }

    pub(crate) fn pad(&mut self, n: usize, style: Style) {
        const SPACES: &str = "                                                                ";
        let mut n = n;
        while n > 0 {
            let k = n.min(SPACES.len());
            self.push(&SPACES[..k], style);
            n -= k;
        }
    }

    /// Repeats `pattern` across `cols` columns, padding any remainder with spaces.
    pub(crate) fn fill(&mut self, pattern: &str, cols: usize, style: Style) {
        let w = pattern.width();
        if w == 0 {
            return self.pad(cols, style);
        }
        for _ in 0..cols / w {
            self.push(pattern, style);
        }
        self.pad(cols % w, style);
    }

    pub(crate) fn end_line(&mut self) {
        let offset = |n: usize| u32::try_from(n).expect("document exceeds 4 GiB of laid-out text");
        self.lines.push(LineMeta {
            text_end: offset(self.text.len()),
            runs_end: offset(self.runs.len()),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::BOLD;

    #[test]
    fn storage_matches_worked_example() {
        let mut doc = Document::new();
        let bold = Style {
            attrs: BOLD,
            ..Style::default()
        };
        doc.push("Hello ", Style::default());
        doc.push("wor", bold);
        doc.push("ld", bold);
        doc.end_line();
        doc.end_line();
        doc.push("Bye", Style::default());
        doc.end_line();
        assert_eq!(doc.text, "Hello worldBye");
        assert_eq!(doc.runs.len(), 3);
        assert_eq!(doc.line(0).0, "Hello world");
        assert_eq!(doc.line(0).1[1], Run { at: 6, style: bold });
        assert_eq!(doc.line(1), ("", &[][..]));
        assert_eq!(doc.line(2).0, "Bye");

        let cap = doc.text.capacity();
        doc.clear();
        assert_eq!(doc.len(), 0);
        assert!(doc.text.capacity() >= cap);
    }
}
