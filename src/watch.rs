use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// How often the pager checks a watched file while idle.
pub const INTERVAL: Duration = Duration::from_millis(250);

#[derive(Debug, PartialEq, Eq)]
pub enum Change {
    None,
    Modified,
    Missing,
}

/// Detects changes to a file from its metadata: one `stat` per check, no thread.
/// The inode catches editors that save by renaming a new file into place.
pub struct Watch {
    path: PathBuf,
    stamp: Option<Stamp>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Stamp {
    modified: Option<SystemTime>,
    len: u64,
    inode: u64,
}

impl Watch {
    pub fn new(path: &Path) -> Self {
        Watch {
            path: path.to_owned(),
            stamp: stamp(path),
        }
    }

    /// Reports what happened since the last check, each change once.
    pub fn check(&mut self) -> Change {
        let now = stamp(&self.path);
        if now == self.stamp {
            return Change::None;
        }
        self.stamp = now;
        if now.is_some() {
            Change::Modified
        } else {
            Change::Missing
        }
    }
}

/// Where the text at `old[start..end]` is in `new`: the occurrence nearest `start`, or
/// `start` itself when the text is gone.
pub fn relocate(old: &str, new: &str, start: usize, end: usize) -> usize {
    let mut end = end.min(old.len()).min(start + 256);
    while !old.is_char_boundary(end) {
        end -= 1;
    }
    let key = old.get(start..end).unwrap_or("").trim_end();
    if key.is_empty() {
        return start.min(new.len());
    }
    new.match_indices(key)
        .map(|(at, _)| at)
        .min_by_key(|&at| at.abs_diff(start))
        .unwrap_or(start.min(new.len()))
}

fn stamp(path: &Path) -> Option<Stamp> {
    let meta = fs::metadata(path).ok()?;
    Some(Stamp {
        modified: meta.modified().ok(),
        len: meta.len(),
        inode: inode(&meta),
    })
}

#[cfg(unix)]
fn inode(meta: &Metadata) -> u64 {
    std::os::unix::fs::MetadataExt::ino(meta)
}

#[cfg(not(unix))]
fn inode(_: &Metadata) -> u64 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mar-watch-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    #[test]
    fn relocate_follows_moved_text() {
        let old = "# T\n\nalpha\n\nbeta\n";
        let start = old.find("beta").unwrap();
        let new = "# T\n\nintro\n\nalpha\n\nbeta\n";
        assert_eq!(
            relocate(old, new, start, old.len()),
            new.find("beta").unwrap()
        );
        let twice = "beta\n\n# T\n\nalpha\n\nbeta\n";
        assert_eq!(
            relocate(old, twice, start, old.len()),
            twice.rfind("beta").unwrap()
        );
        assert_eq!(relocate(old, "gone", start, old.len()), 4);
    }

    #[test]
    fn reports_each_change_once() {
        let path = temp("edit.md");
        fs::write(&path, "one").unwrap();
        let mut watch = Watch::new(&path);
        assert_eq!(watch.check(), Change::None);
        fs::write(&path, "one two").unwrap();
        assert_eq!(watch.check(), Change::Modified);
        assert_eq!(watch.check(), Change::None);
    }

    #[test]
    fn rename_saves_and_deletion() {
        let path = temp("rename.md");
        fs::write(&path, "abc").unwrap();
        let mut watch = Watch::new(&path);
        let replacement = temp("rename.md.tmp");
        fs::write(&replacement, "xyz").unwrap();
        fs::rename(&replacement, &path).unwrap();
        assert_eq!(watch.check(), Change::Modified);
        fs::remove_file(&path).unwrap();
        assert_eq!(watch.check(), Change::Missing);
        assert_eq!(watch.check(), Change::None);
        fs::write(&path, "back").unwrap();
        assert_eq!(watch.check(), Change::Modified);
    }
}
