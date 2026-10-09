//! Downloads, decoding and encoding run on a worker thread, so they never stall the pager.

use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{ExtendedColorType, ImageEncoder, ImageReader, Limits, RgbaImage};

use super::detect::Protocol;
use super::{halves, iterm, sixel};
use crate::render::base64;
use crate::{process, safe};

const MAX_ALLOC: u64 = 256 << 20;
const MAX_DOWNLOAD: u64 = 16 << 20;
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Location {
    File(PathBuf),
    Remote(String),
}

/// One image at one size in cells of `cell` pixels.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Key {
    pub id: u32,
    pub cols: u16,
    pub rows: u16,
    pub cell: (u32, u32),
}

/// An image ready to draw.
pub enum Encoded {
    /// Base64 PNG, sent to kitty once and then placed by id.
    Png(String),
    /// One escape sequence per row of cells, drawn at the start of that row.
    Rows(Vec<String>),
}

enum Job {
    Fetch(u32, String),
    Prepare(Key, Location, Protocol),
}

pub enum Done {
    /// Pixel size of a downloaded image; `None` if it couldn't be fetched or read.
    Size(u32, Option<(u32, u32)>),
    Ready(Key, Option<Encoded>),
}

pub struct Worker {
    jobs: Sender<Job>,
    done: Receiver<Done>,
}

impl Worker {
    pub fn new() -> Self {
        let (jobs, inbox) = mpsc::channel();
        let (outbox, done) = mpsc::channel();
        thread::spawn(move || serve(&inbox, &outbox));
        Worker { jobs, done }
    }

    pub fn fetch(&self, id: u32, url: String) {
        let _ = self.jobs.send(Job::Fetch(id, url));
    }

    pub fn prepare(&self, key: Key, location: Location, protocol: Protocol) {
        let _ = self.jobs.send(Job::Prepare(key, location, protocol));
    }

    pub fn next(&self) -> Option<Done> {
        self.done.try_recv().ok()
    }
}

fn serve(jobs: &Receiver<Job>, done: &Sender<Done>) {
    let mut downloads = HashMap::new();
    for job in jobs {
        let reply = match job {
            Job::Fetch(id, url) => {
                let data = download(&url);
                let size = data.as_deref().and_then(|d| dimensions(reader(d)?));
                if let (Some(data), Some(_)) = (data, size) {
                    downloads.insert(id, data);
                }
                Done::Size(id, size)
            }
            Job::Prepare(key, location, protocol) => {
                let file;
                let data = match &location {
                    Location::File(path) => {
                        file = fs::read(path).ok();
                        file.as_deref()
                    }
                    Location::Remote(_) => downloads.get(&key.id).map(Vec::as_slice),
                };
                let encoded = data.and_then(|data| {
                    safe::catch(AssertUnwindSafe(|| encode(data, key, protocol))).flatten()
                });
                Done::Ready(key, encoded)
            }
        };
        if done.send(reply).is_err() {
            break;
        }
    }
}

/// Pixel size of a local image, read from its header.
pub fn size(path: &Path) -> Option<(u32, u32)> {
    dimensions(ImageReader::open(path).ok()?.with_guessed_format().ok()?)
}

fn reader(data: &[u8]) -> Option<ImageReader<Cursor<&[u8]>>> {
    ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .ok()
}

fn dimensions<R: std::io::BufRead + std::io::Seek>(reader: ImageReader<R>) -> Option<(u32, u32)> {
    reader
        .into_dimensions()
        .ok()
        .filter(|&(w, h)| w > 0 && h > 0)
}

fn download(url: &str) -> Option<Vec<u8>> {
    let mut curl = Command::new("curl");
    curl.args(["--silent", "--fail", "--location", "--max-time", "10"])
        .args(["--max-filesize", "16M", "--proto", "=http,https"])
        .args(["--proto-redir", "=http,https"])
        .arg(url);
    process::run(curl, Vec::new(), DOWNLOAD_TIMEOUT, MAX_DOWNLOAD)
}

/// Decodes `data` and draws it into the top-left of its `key.cols` × `key.rows` cells.
fn encode(data: &[u8], key: Key, protocol: Protocol) -> Option<Encoded> {
    let mut reader = reader(data)?;
    let mut limits = Limits::default();
    limits.max_alloc = Some(MAX_ALLOC);
    reader.limits(limits);
    let image = reader.decode().ok()?;
    let (cell_w, cell_h) = key.cell;
    let (w, h) = (u32::from(key.cols) * cell_w, u32::from(key.rows) * cell_h);
    let image = if image.width() > w || image.height() > h {
        image.thumbnail(w, h)
    } else {
        image
    };
    let mut canvas = RgbaImage::new(w, h);
    image::imageops::overlay(&mut canvas, &image.to_rgba8(), 0, 0);
    Some(match protocol {
        Protocol::Kitty => {
            let mut data = String::new();
            base64(&png(&canvas)?, &mut data);
            Encoded::Png(data)
        }
        Protocol::Iterm => Encoded::Rows(iterm::encode(&canvas, key.cols, cell_h)?),
        Protocol::Sixel => Encoded::Rows(sixel::encode(&canvas, cell_h)),
        Protocol::Halves => Encoded::Rows(halves::encode(&canvas, key.cols, key.rows)),
    })
}

pub fn png(image: &RgbaImage) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    PngEncoder::new_with_quality(&mut out, CompressionType::Fast, FilterType::Adaptive)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ExtendedColorType::Rgba8,
        )
        .ok()?;
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn sample() -> Vec<u8> {
        png(&RgbaImage::from_pixel(16, 8, Rgba([200, 30, 30, 255]))).unwrap()
    }

    #[test]
    fn sizes_come_from_headers() {
        assert_eq!(dimensions(reader(&sample()).unwrap()), Some((16, 8)));
        assert!(reader(b"not an image").and_then(dimensions).is_none());
    }

    #[test]
    fn every_protocol_encodes_one_entry_per_row() {
        let key = Key {
            id: 0,
            cols: 4,
            rows: 2,
            cell: (4, 8),
        };
        for protocol in [Protocol::Iterm, Protocol::Sixel, Protocol::Halves] {
            match encode(&sample(), key, protocol) {
                Some(Encoded::Rows(rows)) => assert_eq!(rows.len(), 2, "{protocol:?}"),
                _ => panic!("{protocol:?} failed"),
            }
        }
        assert!(matches!(
            encode(&sample(), key, Protocol::Kitty),
            Some(Encoded::Png(data)) if data.starts_with("iVBORw0KGgo")
        ));
        assert!(encode(b"garbage", key, Protocol::Sixel).is_none());
    }

    #[test]
    fn worker_prepares_local_files() {
        let path = std::env::temp_dir().join(format!("mar-image-{}.png", std::process::id()));
        fs::write(&path, sample()).unwrap();
        assert_eq!(size(&path), Some((16, 8)));
        let worker = Worker::new();
        let key = Key {
            id: 1,
            cols: 4,
            rows: 1,
            cell: (4, 8),
        };
        worker.prepare(key, Location::File(path.clone()), Protocol::Halves);
        let done = worker.done.recv_timeout(Duration::from_secs(5)).unwrap();
        fs::remove_file(&path).unwrap();
        assert!(matches!(done, Done::Ready(k, Some(Encoded::Rows(_))) if k == key));
    }
}
