use unicode_width::UnicodeWidthStr;

use crate::highlight;
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
    anchors: Vec<(u32, usize)>, // (line, source offset) at each block start
    code: String,
    slugs: String,
    syntax: highlight::Cache, // survives `clear`, so re-layouts don't re-highlight
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

    pub fn source_offset(&self, line: usize) -> usize {
        let k = self.anchors.partition_point(|a| a.0 as usize <= line);
        k.checked_sub(1).map_or(0, |k| self.anchors[k].1)
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
        self.anchors.clear();
        self.code.clear();
        self.slugs.clear();
    }

    pub fn layout(&mut self, src: &str, width: usize, theme: &Theme) {
        self.clear();
        let mut syntax = std::mem::take(&mut self.syntax);
        layout::build(self, &mut syntax, src, width, theme);
        syntax.truncate(self.code_blocks.len());
        self.syntax = syntax;
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
