# marustdown — Design Plan

A terminal markdown viewer in Rust. One file, full screen, status bar. Small dependency list,
low memory, no frame-time surprises. Nothing in here exists unless it shows up in §1.

---

## 1. Target Output

```
        ██ Usage
        ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

        Run the server with your config file. See the configuration
        section↗ for all options.

        ╭─  rust ─────────────────────────────────────── [y] copy ─╮
        │ 1  fn main() {                                            │
        │ 2      let cfg = Config::load("app.toml")?;               │
        │ 3      server::run(cfg).await                             │
        │ 4  }                                                      │
        ╰───────────────────────────────────────────────────────────╯

        ▌ 󰋽 NOTE
        ▌ Requires Rust 1.80 or newer.

        ┌──────────┬─────────┬──────────────────────────────────┐
        │ Flag     │ Default │ Description                      │
        ├──────────┼─────────┼──────────────────────────────────┤
        │ --port   │ 8080    │ Port to listen on                │
        │ --watch  │ false   │ Reload when files change         │
        └──────────┴─────────┴──────────────────────────────────┘

        ✔ Write docs    ☐ Add tests    ☐ Ship it

  ln 84/200 │ Usage › Configuration          / search  n next  ]] next heading  y copy  q quit
```

---

## 2. Shape

Three layers, one type across each boundary, arrows only point right.

```
  path/stdin         &str            Document          bytes
 ──────────► source ──────► doc ──────────────► render ──────► stdout
   memmap2           pure: no terminal        no markdown
```

Two rules keep it that way:

1. `doc` emits no escape codes and never reads the terminal size — width is an argument. It is
   a pure `&str -> Document`, so it tests with plain string asserts.
2. `render`/`pager` never touch `pulldown-cmark`. They only know `Document`.

### Files

Nine flat files, no subdirectories.

```
src/
  main.rs        args, TTY check, pick cat or pager
  source.rs      mmap the file (or slurp stdin) -> &str
  theme.rs       palette, glyphs, Style
  doc.rs         Document + the event loop (headings, lists, quotes, code)
  wrap.rs        find line breaks for a width
  table.rs       GFM table -> lines
  highlight.rs   code token scanner
  render.rs      Document line -> ANSI/OSC bytes
  pager.rs       raw mode, scroll, search
```

| File | May use |
|---|---|
| `source`, `theme` | nothing from this crate |
| `wrap`, `highlight` | `theme` |
| `doc`, `table` | `theme`, `wrap`, `highlight`, `pulldown-cmark` |
| `render`, `pager` | `doc`, `theme`, `crossterm` |

---

## 3. Input (`source.rs`)

`pulldown-cmark` only takes `&'input str`. There is no `BufRead` API and there can't be one:
events borrow from the buffer (`CowStr::Borrowed`) and markdown needs arbitrary lookahead
(reference definitions can come after their use, setext headings need the next line). The whole
source has to be resident — mmap makes it resident without a heap copy, and lets the kernel page
it in on demand and evict it again.

```rust
pub enum Source {
    Mapped(memmap2::Mmap),
    Buffered(String),       // stdin, or files too small for mmap to pay off
}

const MMAP_MIN: u64 = 64 * 1024;   // under this, read() beats mmap + page faults

impl Source {
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        if file.metadata()?.len() < MMAP_MIN {
            let mut s = String::new();
            (&file).read_to_string(&mut s)?;
            return Ok(Source::Buffered(s));
        }
        let map = unsafe { memmap2::Mmap::map(&file)? };
        std::str::from_utf8(&map).map_err(invalid_data)?;   // validated once, SIMD, ~GB/s
        Ok(Source::Mapped(map))
    }

    pub fn text(&self) -> &str {
        match self {
            Source::Mapped(m) => unsafe { std::str::from_utf8_unchecked(m) }, // checked in open
            Source::Buffered(s) => s,
        }
    }
}
```

- `Mmap::map` is `unsafe` because another process truncating the file while it's mapped gives
  you SIGBUS. Acceptable for a viewer; say so in the comment rather than pretending.
- `MMAP_MIN` also covers empty files, where `Mmap::map` errors on some platforms.
- `text()` borrows `self`, so keep `Source` in a `main` local and pass `&str` down. `Document`
  is fully owned, so the lifetime stops there and no other file needs a lifetime parameter.

Parser options — enable only what §1 renders, since each one costs parse time:

```rust
Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH
    | Options::ENABLE_TASKLISTS | Options::ENABLE_GFM   // GFM is what fills BlockQuoteKind
```

Use the plain `Parser`, not `into_offset_iter()` — source ranges are only useful for incremental
reload, which isn't happening.

---

## 4. Document (`doc.rs`)

This is the one place where the memory and speed of the whole program is decided, so it gets
the attention. Everything else is straightforward code.

A rendered document is text plus style changes. The obvious model —
`Vec<Line { spans: Vec<Span { text: String, .. }> }>` — costs two allocations per line plus one
per span. A 5 MB file is roughly 100k lines and 300k spans: about 400k live allocations and
~40 MB once allocator overhead is counted. Every resize frees and re-allocates all of it.

Three flat buffers do the same job in **three** allocations, about 9 MB, and reuse their
capacity on resize:

```rust
// theme.rs
pub type Color = u8;   // index into Theme::palette; 0 = terminal default

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Style {
    pub fg: Color,
    pub bg: Color,
    pub attrs: u8,   // BOLD | DIM | ITALIC | UNDERLINE | STRIKE bitflags
    pub link: u16,   // index into Document::links; 0 = none
}                    // 6 bytes, Copy, compares in one shot

// doc.rs
#[derive(Clone, Copy)]
pub struct Run { pub at: u16, pub style: Style }   // 8 bytes; `at` is relative to line start

struct LineMeta { text_end: u32, runs_end: u32 }   // 8 bytes; start = previous line's end

pub struct Document {
    text:  String,          // every line's plain text, concatenated, no escapes
    runs:  Vec<Run>,        // a style holds until the next Run
    lines: Vec<LineMeta>,
    pub links:       Vec<String>,   // OSC 8 targets, indexed by Style::link
    pub headings:    Vec<Heading>,  // { line: u32, level: u8, parent: u32 }
    pub code_blocks: Vec<String>,   // raw text for `y`; few of them, so plain Strings are fine
}
```

Palette indices instead of `(u8, u8, u8)` shrink `Style` to 6 bytes, make `--no-color` a
one-line swap, and make the theme a single table.

Read side — two lines of arithmetic:

```rust
impl Document {
    pub fn len(&self) -> usize { self.lines.len() }

    pub fn line(&self, i: usize) -> (&str, &[Run]) {
        let (t0, r0) = match i.checked_sub(1) {
            Some(p) => (self.lines[p].text_end as usize, self.lines[p].runs_end as usize),
            None => (0, 0),
        };
        let m = &self.lines[i];
        (&self.text[t0..m.text_end as usize], &self.runs[r0..m.runs_end as usize])
    }
}
```

Write side — `push` merges adjacent equal styles, `end_line` closes a line:

```rust
impl Document {
    fn push(&mut self, s: &str, style: Style) {
        let (line_start, run_start) = self.open_line();     // ends of the previous LineMeta
        if self.runs.len() == run_start || self.runs.last().unwrap().style != style {
            let at = (self.text.len() - line_start) as u16;
            self.runs.push(Run { at, style });
        }
        self.text.push_str(s);
    }

    fn end_line(&mut self) {
        self.lines.push(LineMeta {
            text_end: self.text.len() as u32,
            runs_end: self.runs.len() as u32,
        });
    }

    /// Reuse every buffer; nothing is freed, so a resize allocates nothing.
    pub fn clear(&mut self) {
        self.text.clear(); self.runs.clear(); self.lines.clear();
        self.links.clear(); self.headings.clear(); self.code_blocks.clear();
    }
}
```

Three things fall out of `text` being plain:

- **Width** is `UnicodeWidthStr::width(text)` directly — no escape skipping.
- **Search** is `text.find(needle)` directly. Highlighting splits the run at the match offset at
  render time; nothing is stored.
- **Breadcrumbs** are rebuilt by walking `headings[..].parent` up from the heading at or above
  the top visible line. No per-line `Vec<String>` to clone.

### The event loop

One `Vec<Frame>` stack handles all nesting, so there are no "am I in a quote inside a list"
special cases:

```rust
struct Frame {
    first: (String, Style),   // prefix for the frame's first line: "  • " or "▌ "
    cont:  (String, Style),   // prefix for every line after it:    "    " or "▌ "
}
```

`Start(BlockQuote | List | Item)` pushes, the matching `End` pops. A line's prefix is the stack
concatenated; the text width is `width - stack_width()`. Blockquotes in lists, alerts in quotes
and nested bullets all work with no extra code.

Inline events accumulate into two reusable scratch buffers (`String` + `Vec<Run>`) that are
cleared per block, never reallocated. At `End(Paragraph)` the scratch is wrapped and copied into
the arena.

---

## 5. What Gets Drawn

**Headings** — H1 `██ ` bold accent plus a full-width `━` rule; H2 `▌ ` bold secondary plus a
`─` rule; H3–H6 bold and progressively dimmer, no rule. Blank line either side.

**Paragraphs** — wrapped to `min(term_width - 2 * margin, --width)`, default 90, centered.

**Inline** — bold, italic, strikethrough. `` `code` `` gets accent fg plus a bg tint and one
space of padding each side. Links get underline, an OSC 8 target, and a dim `↗`. Images render
as dim `🖼 alt`.

**Code blocks** — `╭─  rust ─── [y] copy ─╮`, dim right-aligned line numbers, bg tint to the box
edge. Over-long lines truncate with `…`.

**Alerts and quotes** — plain quote is a dim `▌` bar in italic. `BlockQuoteKind` from
`ENABLE_GFM` gives the five alerts: NOTE blue, TIP green, IMPORTANT purple, WARNING yellow,
CAUTION red — colored bar, icon, bold uppercase title.

**Lists** — `•`, `◦`, `▪` by depth, repeating. Ordered numbers right-aligned. Tasks `✔` green /
`☐` dim. Hanging indent comes from `Frame::cont`.

**Tables** — cells to plain text, natural widths, shrink the widest column until it fits, then
wrap. `┌┬┐ ├┼┤ └┴┘ │ ─`, bold header, GFM alignment respected.

**Rule** — a dim `─` across the content width.

**Glyphs** — `char` constants in `theme.rs` with an ASCII fallback set behind `--no-icons`.

### Wrapping (`wrap.rs`)

Keep it a pure function over plain text so it needs no knowledge of styles at all:

```rust
/// Appends byte offsets where `text` should break to fit `width` columns.
/// Reuses `breaks`; allocates nothing.
pub fn breaks(text: &str, width: usize, breaks: &mut Vec<u32>);
```

Greedy fill, measured with `UnicodeWidthStr::width` and never `len()`. Hard-split only words
longer than a whole line. The caller slices the scratch buffer at those offsets and carries the
runs across. Testable with plain `assert_eq!` on offsets.

### Highlighting (`highlight.rs`)

A ~150-line char scanner, no dependency, covering most of the visual effect: comments (`//`,
`#`, `--`, `/* */`) dim italic, strings green, numbers orange, a small `&[&str]` keyword table
per language (rust, python, js/ts, go, bash, json) purple bold, capitalized words yellow, the
rest default. It writes `Run`s straight into the document.

---

## 6. Output (`render.rs`)

```rust
pub fn line(doc: &Document, i: usize, theme: &Theme, out: &mut String) {
    let (text, runs) = doc.line(i);
    for (k, r) in runs.iter().enumerate() {
        let end = runs.get(k + 1).map_or(text.len(), |n| n.at as usize);
        sgr(r.style, theme, out);                      // only what changed since the last run
        if r.style.link != 0 { osc8_open(&doc.links[r.style.link as usize - 1], out); }
        out.push_str(&text[r.at as usize..end]);
        if r.style.link != 0 { out.push_str("\x1b]8;;\x1b\\"); }
    }
    out.push_str("\x1b[0m");
}
```

Speed rules, all cheap:

- One reusable `String` per frame. `buf.clear()` keeps the capacity, so after the first frame
  the pager allocates nothing. One `write_all` plus one flush per frame — no flicker, no
  per-line syscalls.
- Emit SGR only for the attributes that differ from the previous run.
- Render only `lines[scroll .. scroll + height - 1]`.

Clipboard is OSC 52 with a hand-rolled base64 (about 20 lines) — it works over SSH, unlike
`arboard`. tmux needs `set -g set-clipboard on`.

```rust
pub fn clipboard(text: &str) -> String { format!("\x1b]52;c;{}\x07", base64(text.as_bytes())) }
```

---

## 7. Pager (`pager.rs`)

Raw mode, alternate screen, hidden cursor, all restored on exit **and** from a `panic::set_hook`.
On resize: `doc.clear()`, re-layout at the new width, keep the top heading in view.

| Key | Action |
|---|---|
| `j`/`↓`, `k`/`↑` | Line |
| `Space`/`PgDn`, `b`/`PgUp` | Page |
| `d` / `u` | Half page |
| `g` / `G` | Top / bottom |
| `]]` / `[[` | Next / previous heading |
| `/`, `n`, `N` | Search, next, previous |
| `y` | Copy nearest code block |
| `q` / `Esc` | Quit |

Status bar: `ln 84/200 │ Usage › Configuration` on the left, key hints on the right.

---

## 8. CLI (`main.rs`)

```
md <file.md>          pager
md <file.md> --cat    styled output, exit (automatic when stdout isn't a TTY)
md -                  stdin
  --width <n>         content width (default 90)
  --no-color          plain text
  --no-icons          ASCII glyphs
```

Hand-parsed with `std::env::args()`; pipe detection with `std::io::IsTerminal`. Both are std.

---

## 9. Dependencies

A crate only for what's genuinely hard by hand. Anything that's "write some escape codes" is
hand-written.

```toml
[package]
name = "marustdown"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "md"
path = "src/main.rs"

[dependencies]
pulldown-cmark = { version = "0.13", default-features = false }  # drops the HTML renderer
memmap2        = "0.9"                                           # ~300 LOC, zero deps
crossterm      = { version = "0.29", default-features = false, features = ["events"] }
unicode-width  = "0.2"

[profile.release]
opt-level = "s"
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

Four crates, no features, no optional deps. GFM tables, task lists, strikethrough and alerts are
runtime `Options` flags, not Cargo features. crossterm on Windows also needs `"windows"`.

---

## 10. Not In This Version

Named so they don't creep back in: `syntect`, live reload / `--watch`, footnotes, math,
smart punctuation, images, mouse capture, kitty text sizing, incremental re-layout, and the
header line with the progress bar.

---

## 11. Milestones

1. **Skeleton** — `source` + `theme`, dump events. [src/main.rs](src/main.rs) already does this
   with `read_to_string`; swap in `Source`.
2. **Cat mode** — `doc` (headings, paragraphs, inline, `wrap`, frame stack) + `render`. This
   alone is already a useful styled `cat`.
3. **Blocks** — code boxes, quotes, alerts, lists, tasks, rules.
4. **Tables**.
5. **Pager** — raw mode, scroll, status bar, resize.
6. **Polish** — `highlight`, OSC 8, `y` copy, heading jumps, search.

Because `doc` is pure, steps 2–4 test as `&str -> Document` with no terminal. Keep a
`fn plain(doc: &Document) -> String` that drops styles so the asserts stay readable.
