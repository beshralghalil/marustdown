use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use memmap2::Mmap;

pub enum Source {
    Mapped(Mmap),
    Buffered(String), // stdin, watched files, or files too small for mmap to pay off
}

const MAPPED_MIN: u64 = 64 * 1024; // under this, read() beats mmap + page faults

impl Source {
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        if file.metadata()?.len() < MAPPED_MIN {
            let mut buf = String::new();
            (&file).read_to_string(&mut buf)?;
            return Ok(Source::Buffered(buf));
        }
        // SAFETY: another process truncating the file while it is mapped raises SIGBUS.
        // That is accepted for a viewer; nothing else can make this mapping unsound.
        let map = unsafe { Mmap::map(&file)? };
        std::str::from_utf8(&map).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(Source::Mapped(map))
    }

    /// Reads the whole file into memory; for files that change while shown, where a
    /// mapping could be truncated underneath.
    pub fn read(path: &Path) -> io::Result<Self> {
        Ok(Source::Buffered(std::fs::read_to_string(path)?))
    }

    pub fn stdin() -> io::Result<Self> {
        let mut buf = String::new();
        io::stdin().lock().read_to_string(&mut buf)?;
        Ok(Source::Buffered(buf))
    }

    pub fn text(&self) -> &str {
        let text = match self {
            Source::Buffered(s) => s,
            // SAFETY: validated as UTF-8 in `open`.
            Source::Mapped(m) => unsafe { std::str::from_utf8_unchecked(m) },
        };
        text.strip_prefix('\u{feff}').unwrap_or(text)
    }

    /// Offset of `text()` within the underlying bytes (non-zero when a BOM is skipped).
    pub fn base(&self) -> usize {
        let raw: &[u8] = match self {
            Source::Buffered(s) => s.as_bytes(),
            Source::Mapped(m) => m,
        };
        raw.len() - self.text().len()
    }

    /// Rewrites one ASCII byte at `at` (relative to `text()`); only for in-memory sources.
    pub fn set_byte(&mut self, at: usize, byte: u8) -> bool {
        let base = self.base();
        match self {
            Source::Buffered(s)
                if byte.is_ascii() && s.as_bytes().get(base + at).is_some_and(u8::is_ascii) =>
            {
                let at = base + at;
                s.replace_range(at..at + 1, (byte as char).encode_utf8(&mut [0; 4]));
                true
            }
            _ => false,
        }
    }
}
