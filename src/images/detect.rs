use std::env;

use crate::config::ImageMode;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Protocol {
    Kitty,
    Iterm,
    Sixel,
    Halves,
}

impl Protocol {
    /// Sixel and iTerm2 images are resent in full on every move, so they wait for
    /// scrolling to settle.
    pub fn heavy(self) -> bool {
        matches!(self, Protocol::Sixel | Protocol::Iterm)
    }
}

/// How images reach this terminal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Terminal {
    pub protocol: Protocol,
    pub cell: (u32, u32), // pixels per cell
}

/// Cell size assumed when the terminal doesn't report one.
const CELL: (u32, u32) = (10, 20);

/// Picks the protocol from `mode`, or for `Auto` from the environment and the
/// terminal's answers, the way Yazi does. Expects raw mode, since it reads replies.
pub fn detect(mode: ImageMode, color: bool) -> Option<Terminal> {
    let replies = if mode == ImageMode::Off {
        Replies::default()
    } else {
        parse(&query())
    };
    let cell = replies.cell.or_else(window_cell);
    let protocol = match mode {
        ImageMode::Off => return None,
        ImageMode::Kitty => Protocol::Kitty,
        ImageMode::Iterm => Protocol::Iterm,
        ImageMode::Sixel => Protocol::Sixel,
        ImageMode::Blocks => Protocol::Halves,
        ImageMode::Auto => match choose(|k| env::var(k).ok(), &replies) {
            Some(Protocol::Sixel) if cell.is_none() => Protocol::Halves,
            Some(p) => p,
            None => return None,
        },
    };
    if protocol == Protocol::Halves && !color {
        return None;
    }
    Some(Terminal {
        protocol,
        cell: cell.unwrap_or(CELL),
    })
}

/// Terminals known by their environment come first; then what the terminal said it
/// supports; then half-blocks. Inside tmux and zellij only sixel passes through as is.
fn choose(var: impl Fn(&str) -> Option<String>, replies: &Replies) -> Option<Protocol> {
    let term = var("TERM").unwrap_or_default();
    let program = var("TERM_PROGRAM").unwrap_or_default();
    if term == "dumb" || term == "linux" {
        return None;
    }
    let multiplexed = var("TMUX").is_some() || var("ZELLIJ").is_some();
    let known = if multiplexed {
        None
    } else if var("KITTY_WINDOW_ID").is_some()
        || term == "xterm-kitty"
        || term == "xterm-ghostty"
        || program == "ghostty"
        || var("KONSOLE_VERSION").is_some()
    {
        Some(Protocol::Kitty)
    } else if matches!(
        program.as_str(),
        "iTerm.app" | "WezTerm" | "WarpTerminal" | "mintty" | "Bobcat"
    ) {
        Some(Protocol::Iterm)
    } else if matches!(program.as_str(), "vscode" | "Tabby") {
        // xterm.js draws images only with its image addon on, which also brings sixel.
        replies.sixel.then_some(Protocol::Iterm)
    } else if term.starts_with("foot") {
        Some(Protocol::Sixel)
    } else {
        None
    };
    let answered = if replies.kitty && !multiplexed {
        Some(Protocol::Kitty)
    } else if replies.sixel {
        Some(Protocol::Sixel)
    } else {
        None
    };
    Some(known.or(answered).unwrap_or(Protocol::Halves))
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Replies {
    kitty: bool,
    sixel: bool,
    cell: Option<(u32, u32)>,
}

/// A kitty graphics query, the cell size in pixels, then primary device attributes,
/// which every terminal answers, so its reply marks the end.
#[cfg(unix)]
const QUERY: &[u8] = b"\x1b_Gi=31,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\\x1b[16t\x1b[c";

#[cfg(unix)]
fn query() -> Vec<u8> {
    use std::fs::OpenOptions;
    use std::io::{Read, Write};
    use std::os::fd::AsRawFd;
    use std::time::{Duration, Instant};

    let Ok(mut tty) = OpenOptions::new().read(true).write(true).open("/dev/tty") else {
        return Vec::new();
    };
    if tty.write_all(QUERY).and_then(|()| tty.flush()).is_err() {
        return Vec::new();
    }
    let deadline = Instant::now() + Duration::from_millis(500);
    let (mut reply, mut buf) = (Vec::new(), [0; 256]);
    while attributes(&String::from_utf8_lossy(&reply)).is_none() {
        let left = deadline
            .saturating_duration_since(Instant::now())
            .as_millis();
        let mut fd = libc::pollfd {
            fd: tty.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: `fd` is a single valid pollfd that outlives the call.
        if unsafe { libc::poll(&mut fd, 1, left.min(1000) as libc::c_int) } <= 0 {
            break;
        }
        match tty.read(&mut buf) {
            Ok(n) if n > 0 => reply.extend_from_slice(&buf[..n]),
            _ => break,
        }
    }
    reply
}

#[cfg(not(unix))]
fn query() -> Vec<u8> {
    Vec::new()
}

fn parse(reply: &[u8]) -> Replies {
    let text = String::from_utf8_lossy(reply);
    let cell = between(&text, "\x1b[6;", 't').and_then(|size| {
        let (h, w) = size.split_once(';')?;
        let (w, h) = (w.parse().ok()?, h.parse().ok()?);
        (w > 0 && h > 0).then_some((w, h))
    });
    Replies {
        kitty: text.contains("\x1b_Gi=31;OK"),
        sixel: attributes(&text).is_some_and(|a| a.split(';').any(|p| p == "4")),
        cell,
    }
}

/// Parameters of the primary device attributes reply.
fn attributes(text: &str) -> Option<&str> {
    between(text, "\x1b[?", 'c')
}

fn between<'t>(text: &'t str, start: &str, end: char) -> Option<&'t str> {
    let rest = &text[text.find(start)? + start.len()..];
    let body = &rest[..rest.find(end)?];
    body.bytes()
        .all(|b| b.is_ascii_digit() || b == b';')
        .then_some(body)
}

/// Cell size from the window size in pixels, where the terminal reports one.
pub fn window_cell() -> Option<(u32, u32)> {
    let size = crossterm::terminal::window_size().ok()?;
    let (cols, rows) = (u32::from(size.columns), u32::from(size.rows));
    let (w, h) = (u32::from(size.width), u32::from(size.height));
    (cols > 0 && rows > 0 && w >= cols && h >= rows).then(|| (w / cols, h / rows))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pick(vars: &[(&str, &str)], replies: &Replies) -> Option<Protocol> {
        choose(
            |k| {
                vars.iter()
                    .find(|(name, _)| *name == k)
                    .map(|(_, v)| v.to_string())
            },
            replies,
        )
    }

    #[test]
    fn parses_replies() {
        let reply = b"\x1b_Gi=31;OK\x1b\\\x1b[6;20;10t\x1b[?62;4;22c";
        assert_eq!(
            parse(reply),
            Replies {
                kitty: true,
                sixel: true,
                cell: Some((10, 20)),
            }
        );
        assert_eq!(parse(b"\x1b[?1;2c"), Replies::default());
        assert!(!parse(b"\x1b[?64;14c").sixel);
        assert_eq!(parse(b"\x1b[6;0;0t\x1b[?4c").cell, None);
        assert_eq!(parse(b""), Replies::default());
    }

    #[test]
    fn known_terminals_win() {
        let none = Replies::default();
        let sixel = Replies {
            sixel: true,
            ..Replies::default()
        };
        assert_eq!(
            pick(&[("TERM", "xterm-kitty")], &none),
            Some(Protocol::Kitty)
        );
        assert_eq!(
            pick(&[("TERM_PROGRAM", "ghostty")], &none),
            Some(Protocol::Kitty)
        );
        assert_eq!(
            pick(&[("TERM_PROGRAM", "WezTerm")], &sixel),
            Some(Protocol::Iterm)
        );
        assert_eq!(pick(&[("TERM", "foot")], &none), Some(Protocol::Sixel));
        assert_eq!(pick(&[("TERM", "linux")], &sixel), None);
        let vscode = [("TERM_PROGRAM", "vscode")];
        assert_eq!(pick(&vscode, &sixel), Some(Protocol::Iterm));
        assert_eq!(pick(&vscode, &none), Some(Protocol::Halves));
    }

    #[test]
    fn replies_decide_for_unknown_terminals() {
        let kitty = Replies {
            kitty: true,
            sixel: true,
            ..Replies::default()
        };
        let sixel = Replies {
            sixel: true,
            ..Replies::default()
        };
        let term = [("TERM", "xterm-256color")];
        assert_eq!(pick(&term, &kitty), Some(Protocol::Kitty));
        assert_eq!(pick(&term, &sixel), Some(Protocol::Sixel));
        assert_eq!(pick(&term, &Replies::default()), Some(Protocol::Halves));
    }

    #[test]
    fn multiplexers_get_sixel_or_half_blocks() {
        let kitty = Replies {
            kitty: true,
            ..Replies::default()
        };
        let sixel = Replies {
            sixel: true,
            ..Replies::default()
        };
        let tmux = [("TMUX", "/tmp/tmux"), ("TERM", "xterm-kitty")];
        assert_eq!(pick(&tmux, &kitty), Some(Protocol::Halves));
        assert_eq!(pick(&tmux, &sixel), Some(Protocol::Sixel));
        assert_eq!(
            pick(&[("ZELLIJ", "0"), ("TERM_PROGRAM", "WezTerm")], &kitty),
            Some(Protocol::Halves)
        );
    }
}
