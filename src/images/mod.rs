//! Images in the pager, drawn with kitty graphics, iTerm2 inline images, sixel or
//! half-blocks, whichever the terminal supports. The layout leaves blank lines for each
//! picture; `draw` paints over them after the text and only when a picture moved.

mod detect;
mod halves;
mod iterm;
mod kitty;
mod load;
mod sixel;

use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use crate::config::Layout;
use crate::doc::Document;
use crate::layout::{Fit, Pictures};
use crate::links::{self, Target};
use detect::{Protocol, Terminal};
use load::{Done, Encoded, Key, Location, Worker};

/// Prepared images kept for reuse; the least recently drawn go first.
const KEEP: usize = 32;

/// What finished background work asks of the pager.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Update {
    None,
    Redraw,
    Relayout,
}

/// The part of the document on screen.
pub struct View {
    pub top: usize,
    pub rows: usize,
    pub margin: usize,
    pub screen_rows: usize,
    pub moving: bool,
}

pub struct Images {
    terminal: Terminal,
    remote: bool,
    height: usize, // percent of the window an image may take
    max_rows: usize,
    base: PathBuf,
    ids: HashMap<Location, u32>,
    sources: Vec<(Location, Size)>,
    worker: Worker,
    pending: usize,
    slots: HashMap<Key, Slot>,
    clock: u64,
    numbers: u32, // kitty image ids handed out
    shown: Vec<Shown>,
}

#[derive(Clone, Copy)]
enum Size {
    Loading,
    Known(u32, u32),
    Failed,
}

enum Slot {
    Pending,
    Ready(Ready),
}

struct Ready {
    encoded: Encoded,
    number: u32,
    sent: bool,
    used: u64,
}

/// Rows `skip..skip + rows` of picture `index`, drawn from screen row `row`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Placement {
    key: Key,
    index: u32,
    row: u16,
    col: u16,
    skip: u16,
    rows: u16,
}

struct Shown {
    at: Placement,
    drawn: bool,
}

impl Images {
    /// `None` when images are off or the terminal can't show them. Expects raw mode.
    pub fn new(layout: &Layout) -> Option<Self> {
        let terminal = detect::detect(layout.images, layout.color)?;
        Some(Self::with_terminal(terminal, layout))
    }

    fn with_terminal(terminal: Terminal, layout: &Layout) -> Self {
        Images {
            terminal,
            remote: layout.remote_images,
            height: layout.image_height,
            max_rows: 1,
            base: PathBuf::new(),
            ids: HashMap::new(),
            sources: Vec::new(),
            worker: Worker::new(),
            pending: 0,
            slots: HashMap::new(),
            clock: 0,
            numbers: 0,
            shown: Vec::new(),
        }
    }

    /// Relative paths resolve against `base`, and pictures fit a window `rows` tall.
    pub fn set_view(&mut self, base: &Path, rows: usize) {
        base.clone_into(&mut self.base);
        self.max_rows = (rows * self.height / 100).max(1);
    }

    /// Picks up a new cell size after a resize; prepared images then start over.
    pub fn resize(&mut self, out: &mut String) {
        let Some(cell) = detect::window_cell().filter(|&c| c != self.terminal.cell) else {
            return;
        };
        self.terminal.cell = cell;
        if self.terminal.protocol == Protocol::Kitty {
            kitty::delete_all(out);
        }
        self.slots.retain(|_, slot| matches!(slot, Slot::Pending));
        self.shown.clear();
    }

    pub fn loading(&self) -> bool {
        self.pending > 0
    }

    /// Takes finished downloads and encodings.
    pub fn receive(&mut self) -> Update {
        let mut update = Update::None;
        while let Some(done) = self.worker.next() {
            self.pending -= 1;
            update = update.max(match done {
                Done::Size(id, size) => {
                    self.sources[id as usize].1 =
                        size.map_or(Size::Failed, |(w, h)| Size::Known(w, h));
                    if size.is_some() {
                        Update::Relayout
                    } else {
                        Update::None
                    }
                }
                Done::Ready(key, Some(encoded)) => {
                    self.numbers += 1;
                    let ready = Ready {
                        encoded,
                        number: self.numbers,
                        sent: false,
                        used: self.clock,
                    };
                    self.slots.insert(key, Slot::Ready(ready));
                    Update::Redraw
                }
                Done::Ready(key, None) => {
                    self.slots.remove(&key);
                    self.sources[key.id as usize].1 = Size::Failed;
                    Update::Relayout
                }
            });
        }
        update
    }

    /// Forgets what is on screen, for when the text was redrawn over it.
    pub fn reset(&mut self, out: &mut String) {
        if self.terminal.protocol == Protocol::Kitty && self.shown.iter().any(|s| s.drawn) {
            kitty::unplace_all(out);
        }
        self.shown.clear();
    }

    /// Like `reset`, for a terminal that may have dropped its images, as after an editor
    /// ran on the main screen.
    pub fn reenter(&mut self, out: &mut String) {
        self.reset(out);
        for slot in self.slots.values_mut() {
            if let Slot::Ready(ready) = slot {
                ready.sent = false;
            }
        }
    }

    /// Paints the pictures in `view` that aren't on screen yet. Their cells must have
    /// been left alone by the text.
    pub fn draw(&mut self, doc: &Document, view: &View, out: &mut String) {
        let next = self.placements(doc, view);
        let kitty = self.terminal.protocol == Protocol::Kitty;
        for s in self
            .shown
            .iter()
            .filter(|s| s.drawn && !next.contains(&s.at))
        {
            if let (true, Some(Slot::Ready(ready))) = (kitty, self.slots.get(&s.at.key)) {
                kitty::unplace(ready.number, s.at.index + 1, out);
            }
        }
        let shown = next
            .into_iter()
            .map(|at| {
                let kept = self.shown.iter().any(|s| s.drawn && s.at == at);
                let drawn = kept || {
                    erase(&at, view.margin, out);
                    !(view.moving && self.terminal.protocol.heavy())
                        && self.paint(&at, view.margin, out)
                };
                Shown { at, drawn }
            })
            .collect();
        self.shown = shown;
        self.evict(out);
    }

    fn placements(&self, doc: &Document, view: &View) -> Vec<Placement> {
        let (top, mut end) = (view.top, view.top + view.rows);
        if self.terminal.protocol.heavy() {
            // An image on the last row would scroll the screen.
            end = end.min(top + view.screen_rows.saturating_sub(1));
        }
        doc.pictures
            .iter()
            .zip(0..)
            .filter_map(|(p, index)| {
                let first = (p.first as usize).max(top);
                let last = (p.first as usize + p.rows as usize).min(end);
                (first < last).then(|| Placement {
                    key: Key {
                        id: p.id,
                        cols: p.cols,
                        rows: p.rows,
                        cell: self.terminal.cell,
                    },
                    index,
                    row: (first - top) as u16,
                    col: p.col,
                    skip: (first - p.first as usize) as u16,
                    rows: (last - first) as u16,
                })
            })
            .collect()
    }

    /// Draws `at` if its image is ready, otherwise asks the worker for it.
    fn paint(&mut self, at: &Placement, margin: usize, out: &mut String) -> bool {
        self.clock += 1;
        let col = margin + at.col as usize;
        let ready = match self.slots.get_mut(&at.key) {
            Some(Slot::Ready(ready)) => ready,
            Some(Slot::Pending) => return false,
            None => {
                let location = self.sources[at.key.id as usize].0.clone();
                self.worker
                    .prepare(at.key, location, self.terminal.protocol);
                self.slots.insert(at.key, Slot::Pending);
                self.pending += 1;
                return false;
            }
        };
        ready.used = self.clock;
        match &ready.encoded {
            Encoded::Png(data) => {
                if !ready.sent {
                    kitty::transmit(ready.number, data, out);
                    ready.sent = true;
                }
                goto(out, at.row as usize, col);
                kitty::place(
                    ready.number,
                    at.index + 1,
                    at.key.cell,
                    at.key.cols,
                    at.skip,
                    at.rows,
                    out,
                );
            }
            Encoded::Rows(rows) => {
                let visible = rows.iter().skip(at.skip as usize).take(at.rows as usize);
                for (k, row) in visible.enumerate() {
                    goto(out, at.row as usize + k, col);
                    out.push_str(row);
                }
            }
        }
        true
    }

    fn evict(&mut self, out: &mut String) {
        while self.slots.len() > KEEP {
            let shown = &self.shown;
            let Some((&key, number)) = self
                .slots
                .iter()
                .filter(|(key, _)| !shown.iter().any(|s| s.at.key == **key))
                .filter_map(|(key, slot)| match slot {
                    Slot::Ready(ready) => Some((key, ready)),
                    Slot::Pending => None,
                })
                .min_by_key(|(_, ready)| ready.used)
                .map(|(key, ready)| (key, ready.sent.then_some(ready.number)))
            else {
                return;
            };
            self.slots.remove(&key);
            if let Some(number) = number {
                kitty::delete(number, out);
            }
        }
    }

    /// The source behind `url`, registered on first sight. Local sizes are read right
    /// away; remote images are fetched in the background.
    fn source(&mut self, url: &str) -> Option<u32> {
        let location = match links::classify(url, Some(&self.base)) {
            Target::Local { path, .. } => Location::File(path),
            Target::External(url)
                if self.remote && (url.starts_with("https://") || url.starts_with("http://")) =>
            {
                Location::Remote(url.to_owned())
            }
            _ => return None,
        };
        if let Some(&id) = self.ids.get(&location) {
            return Some(id);
        }
        let id = u32::try_from(self.sources.len()).ok()?;
        let size = match &location {
            Location::File(path) => {
                load::size(path).map_or(Size::Failed, |(w, h)| Size::Known(w, h))
            }
            Location::Remote(url) => {
                self.worker.fetch(id, url.clone());
                self.pending += 1;
                Size::Loading
            }
        };
        self.ids.insert(location.clone(), id);
        self.sources.push((location, size));
        Some(id)
    }
}

impl Pictures for Images {
    /// Scales the image down to fit `cols` columns and the height limit, never up.
    fn fit(&mut self, url: &str, cols: usize) -> Option<Fit> {
        let id = self.source(url)?;
        let Size::Known(w, h) = self.sources[id as usize].1 else {
            return None;
        };
        let (cell_w, cell_h) = self.terminal.cell;
        let (w, h) = (f64::from(w), f64::from(h));
        let scale = (cols as f64 * f64::from(cell_w) / w)
            .min(self.max_rows as f64 * f64::from(cell_h) / h)
            .min(1.0);
        let cells =
            |px: f64, cell: u32| ((px * scale).floor().max(1.0) / f64::from(cell)).ceil() as u16;
        Some(Fit {
            id,
            cols: cells(w, cell_w),
            rows: cells(h, cell_h),
        })
    }
}

impl Drop for Images {
    fn drop(&mut self) {
        if self.terminal.protocol == Protocol::Kitty && self.numbers > 0 {
            let mut out = String::new();
            kitty::delete_all(&mut out);
            let mut stdout = io::stdout();
            let _ = stdout
                .write_all(out.as_bytes())
                .and_then(|()| stdout.flush());
        }
    }
}

/// Blanks the cells of `at`, clearing whatever was drawn there before.
fn erase(at: &Placement, margin: usize, out: &mut String) {
    for k in 0..at.rows as usize {
        goto(out, at.row as usize + k, margin + at.col as usize);
        crate::render::pad(out, at.key.cols as usize);
    }
}

fn goto(out: &mut String, row: usize, col: usize) {
    let _ = write!(out, "\x1b[{};{}H", row + 1, col + 1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::test_theme;
    use std::time::{Duration, Instant};

    fn images(protocol: Protocol) -> Images {
        let terminal = Terminal {
            protocol,
            cell: (10, 20),
        };
        let mut images = Images::with_terminal(terminal, &test_theme().layout);
        images.set_view(Path::new(""), 10);
        images
    }

    /// A `w`×`h` PNG in the temp directory.
    fn sample(name: &str, w: u32, h: u32) -> PathBuf {
        let path = std::env::temp_dir().join(format!("mar-{}-{name}.png", std::process::id()));
        let image = image::RgbaImage::from_pixel(w, h, image::Rgba([200, 30, 30, 255]));
        std::fs::write(&path, load::png(&image).unwrap()).unwrap();
        path
    }

    fn document(images: &mut Images, path: &Path) -> Document {
        let mut doc = Document::new();
        let src = format!("![x]({})", path.display());
        doc.layout_with(&src, 40, &test_theme(), Some(images));
        doc
    }

    fn view(top: usize, rows: usize) -> View {
        View {
            top,
            rows,
            margin: 2,
            screen_rows: rows + 1,
            moving: false,
        }
    }

    fn ready(images: &mut Images, doc: &Document, rows: &[&str]) {
        let p = doc.pictures[0];
        let key = Key {
            id: p.id,
            cols: p.cols,
            rows: p.rows,
            cell: (10, 20),
        };
        let ready = Ready {
            encoded: Encoded::Rows(rows.iter().map(|r| r.to_string()).collect()),
            number: 1,
            sent: false,
            used: 0,
        };
        images.slots.insert(key, Slot::Ready(ready));
    }

    #[test]
    fn pictures_scale_down_but_never_up() {
        let mut images = images(Protocol::Halves);
        let (small, wide) = (sample("small", 100, 50), sample("wide", 1000, 100));
        let fit = |images: &mut Images, path: &Path| {
            let f = images.fit(path.to_str().unwrap(), 40)?;
            Some((f.cols, f.rows))
        };
        assert_eq!(fit(&mut images, &small), Some((10, 3)));
        assert_eq!(fit(&mut images, &wide), Some((40, 2)));
        assert_eq!(fit(&mut images, Path::new("/no/such.png")), None);
        assert_eq!(images.fit("https://example.com/a.png", 40), None);
        assert!(!images.loading());
        std::fs::remove_file(small).unwrap();
        std::fs::remove_file(wide).unwrap();
    }

    #[test]
    fn pictures_are_drawn_once_until_they_move() {
        let mut images = images(Protocol::Halves);
        let path = sample("once", 100, 50);
        let doc = document(&mut images, &path);
        std::fs::remove_file(&path).unwrap();
        let mut out = String::new();
        images.draw(&doc, &view(0, 10), &mut out);
        assert!(out.starts_with("\x1b[1;3H          \x1b[2;3H"));
        assert!(images.loading());

        ready(&mut images, &doc, &["a", "b", "c"]);
        out.clear();
        images.draw(&doc, &view(0, 10), &mut out);
        assert!(out.ends_with("\x1b[1;3Ha\x1b[2;3Hb\x1b[3;3Hc"));
        out.clear();
        images.draw(&doc, &view(0, 10), &mut out);
        assert_eq!(out, "");

        images.draw(&doc, &view(1, 10), &mut out);
        assert!(out.ends_with("\x1b[1;3Hb\x1b[2;3Hc"));
        out.clear();
        images.reset(&mut out);
        images.draw(&doc, &view(1, 10), &mut out);
        assert!(out.ends_with("\x1b[1;3Hb\x1b[2;3Hc"));
    }

    #[test]
    fn heavy_protocols_wait_for_scrolling_and_spare_the_last_row() {
        let mut images = images(Protocol::Sixel);
        let path = sample("heavy", 100, 50);
        let doc = document(&mut images, &path);
        std::fs::remove_file(&path).unwrap();
        ready(&mut images, &doc, &["a", "b", "c"]);
        let mut out = String::new();
        let moving = View {
            moving: true,
            ..view(0, 10)
        };
        images.draw(&doc, &moving, &mut out);
        assert!(!out.contains('a'));
        out.clear();
        let full = View {
            screen_rows: 2,
            ..view(0, 2)
        };
        images.draw(&doc, &full, &mut out);
        assert!(out.ends_with("\x1b[1;3Ha"), "{out:?}");
    }

    #[test]
    fn kitty_sends_images_once_and_removes_placements() {
        let mut images = images(Protocol::Kitty);
        let path = sample("kitty", 100, 50);
        let doc = document(&mut images, &path);
        std::fs::remove_file(&path).unwrap();
        let p = doc.pictures[0];
        let key = Key {
            id: p.id,
            cols: p.cols,
            rows: p.rows,
            cell: (10, 20),
        };
        let ready = Ready {
            encoded: Encoded::Png("AAAA".into()),
            number: 5,
            sent: false,
            used: 0,
        };
        images.slots.insert(key, Slot::Ready(ready));
        let mut out = String::new();
        images.draw(&doc, &view(0, 10), &mut out);
        assert!(out.contains("a=t,f=100,t=d,i=5"));
        assert!(out.contains("a=p,i=5,p=1,x=0,y=0,w=100,h=60,c=10,r=3"));
        out.clear();
        images.draw(&doc, &view(1, 10), &mut out);
        assert!(!out.contains("a=t"));
        assert!(out.contains("a=d,d=i,i=5,p=1"));
        assert!(out.contains("y=20,w=100,h=40,c=10,r=2"));
        out.clear();
        images.draw(&doc, &view(5, 10), &mut out);
        assert_eq!(out, "\x1b_Ga=d,d=i,i=5,p=1,q=2\x1b\\");
    }

    #[test]
    fn the_worker_prepares_what_comes_into_view() {
        let mut images = images(Protocol::Halves);
        let path = sample("worker", 100, 50);
        let doc = document(&mut images, &path);
        let mut out = String::new();
        images.draw(&doc, &view(0, 10), &mut out);
        let deadline = Instant::now() + Duration::from_secs(5);
        while images.receive() != Update::Redraw {
            assert!(Instant::now() < deadline, "no image from the worker");
            std::thread::sleep(Duration::from_millis(5));
        }
        std::fs::remove_file(&path).unwrap();
        assert!(!images.loading());
        out.clear();
        images.draw(&doc, &view(0, 10), &mut out);
        assert!(out.contains("\x1b[0;38;2;200;30;30;48;2;200;30;30m▀"));
    }
}
