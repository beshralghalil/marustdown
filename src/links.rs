use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;

/// Home-row keys used for hint tags; lowercase only, since uppercase means "copy".
pub const HINT_CHARS: &[u8] = b"asdfghjkl";

#[derive(Debug, PartialEq, Eq)]
pub enum Target<'a> {
    Anchor(&'a str),
    Local {
        path: PathBuf,
        anchor: Option<&'a str>,
    },
    External(&'a str),
}

/// Sorts a link destination into an in-document anchor, a local file (resolved
/// against `base`, the directory of the current file) or anything else.
pub fn classify<'a>(url: &'a str, base: Option<&Path>) -> Target<'a> {
    if let Some(anchor) = url.strip_prefix('#') {
        return Target::Anchor(anchor);
    }
    if has_scheme(url) {
        return Target::External(url);
    }
    let (path, anchor) = match url.split_once('#') {
        Some((p, a)) => (p, Some(a).filter(|a| !a.is_empty())),
        None => (url, None),
    };
    let path = PathBuf::from(percent_decode(path.split('?').next().unwrap_or(path)));
    let path = match base {
        Some(base) if path.is_relative() => base.join(path),
        _ => path,
    };
    Target::Local { path, anchor }
}

/// `scheme:` per RFC 3986, at least two characters so `C:` paths don't count.
fn has_scheme(url: &str) -> bool {
    url.split_once(':').is_some_and(|(scheme, _)| {
        scheme.len() > 1
            && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
    })
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = b
            .get(i + 1..i + 3)
            .and_then(|h| u8::from_str_radix(std::str::from_utf8(h).ok()?, 16).ok());
        match (b[i], hex) {
            (b'%', Some(byte)) => {
                out.push(byte);
                i += 3;
            }
            (c, _) => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| s.to_owned())
}

pub fn is_markdown(path: &Path) -> bool {
    path.extension().and_then(OsStr::to_str).is_some_and(|e| {
        ["md", "markdown", "mdown", "mkd"]
            .iter()
            .any(|m| e.eq_ignore_ascii_case(m))
    })
}

/// GitHub-style heading anchor: lowercase, punctuation dropped, spaces to dashes.
pub fn slugify(text: &str, out: &mut String) {
    for c in text.chars() {
        if c.is_alphanumeric() || c == '-' || c == '_' {
            out.extend(c.to_lowercase());
        } else if c == ' ' {
            out.push('-');
        }
    }
}

/// `n` distinct tags of equal length, so a tag is complete once its length is typed.
pub fn hint_labels(n: usize) -> Vec<String> {
    let k = HINT_CHARS.len();
    let mut len = 1;
    while k.pow(len) < n {
        len += 1;
    }
    (0..n)
        .map(|mut i| {
            let mut tag = vec![0; len as usize];
            for slot in tag.iter_mut().rev() {
                *slot = HINT_CHARS[i % k];
                i /= k;
            }
            String::from_utf8(tag).expect("ASCII hint chars")
        })
        .collect()
}

/// Hands `target` to the desktop opener without waiting for it.
pub fn open_external(target: &OsStr) -> io::Result<()> {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let mut child = Command::new(opener)
        .arg(target)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    thread::spawn(move || child.wait());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_targets() {
        let base = Path::new("/docs");
        assert_eq!(classify("#usage", Some(base)), Target::Anchor("usage"));
        assert_eq!(
            classify("https://x.io/a#b", Some(base)),
            Target::External("https://x.io/a#b")
        );
        assert_eq!(
            classify("mailto:a@b.c", None),
            Target::External("mailto:a@b.c")
        );
        assert_eq!(
            classify("../guide.md#files", Some(base)),
            Target::Local {
                path: "/docs/../guide.md".into(),
                anchor: Some("files")
            }
        );
        assert_eq!(
            classify("my%20notes.md?raw=1", None),
            Target::Local {
                path: "my notes.md".into(),
                anchor: None
            }
        );
        assert_eq!(
            classify("/abs/x.md", Some(base)),
            Target::Local {
                path: "/abs/x.md".into(),
                anchor: None
            }
        );
    }

    #[test]
    fn slugs_match_github() {
        let slug = |t: &str| {
            let mut s = String::new();
            slugify(t, &mut s);
            s
        };
        assert_eq!(slug("Hello, World!"), "hello-world");
        assert_eq!(slug("3. Input (`source.rs`)"), "3-input-sourcers");
        assert_eq!(slug("Café au lait"), "café-au-lait");
    }

    #[test]
    fn labels_are_unique_and_even() {
        assert_eq!(hint_labels(3), ["a", "s", "d"]);
        let many = hint_labels(20);
        assert!(many.iter().all(|l| l.len() == 2));
        let mut unique = many.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), 20);
        assert!(hint_labels(0).is_empty());
    }

    #[test]
    fn markdown_extensions() {
        assert!(is_markdown(Path::new("a/B.MD")));
        assert!(!is_markdown(Path::new("a.txt")));
    }
}
