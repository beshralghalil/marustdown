use std::collections::{BTreeMap, HashMap};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::iter::Peekable;
use std::panic::AssertUnwindSafe;
use std::process::Command;
use std::time::Duration;

use crate::{config, process, safe};

/// Where a rendered block goes.
pub struct Context<'a> {
    pub lang: &'a str,
    pub width: usize,
    pub ascii: bool,
}

/// Draws the source of a fenced code block as lines of text, shown instead of the code box.
pub trait Render {
    /// Distinguishes renderers in the cache: equal names must mean equal output.
    fn name(&self) -> &str;

    /// `None` when the source can't be drawn; the block then shows as code.
    fn render(&self, source: &str, cx: &Context) -> Option<Vec<String>>;
}

/// Renderers by fence language. Earlier entries win, so configured commands come first.
#[derive(Default)]
pub struct Registry {
    entries: Vec<(String, Box<dyn Render>)>,
}

impl Registry {
    /// The configured commands, then the built-in renderers this build has.
    pub fn new(
        commands: BTreeMap<String, config::Command>,
        timeout: Duration,
    ) -> Result<Self, String> {
        let mut registry = Registry::default();
        for (lang, command) in commands {
            let external = External::new(command.argv(), timeout)
                .ok_or_else(|| format!("renderers.{lang}: empty command"))?;
            registry.add(&lang, Box::new(external));
        }
        #[cfg(feature = "mermaid")]
        registry.add("mermaid", Box::new(crate::mermaid::Mermaid));
        Ok(registry)
    }

    pub fn add(&mut self, lang: &str, renderer: Box<dyn Render>) {
        self.entries.push((lang.to_ascii_lowercase(), renderer));
    }

    pub fn get(&self, lang: &str) -> Option<&dyn Render> {
        self.entries
            .iter()
            .find(|(l, _)| l.eq_ignore_ascii_case(lang))
            .map(|(_, r)| r.as_ref())
    }
}

const CACHE_SIZE: usize = 256;

/// Rendered blocks kept across re-layouts, so a renderer runs once per distinct input.
#[derive(Default)]
pub struct Cache {
    entries: HashMap<u64, Entry>,
    clock: u64,
}

struct Entry {
    lines: Option<Vec<String>>,
    used: u64,
}

impl Cache {
    /// Renders `source`, reusing an earlier result for the same input. A renderer that
    /// panics counts as failed.
    pub fn render(
        &mut self,
        renderer: &dyn Render,
        source: &str,
        cx: &Context,
    ) -> Option<Vec<String>> {
        let mut hasher = DefaultHasher::new();
        (renderer.name(), cx.lang, source, cx.width, cx.ascii).hash(&mut hasher);
        let key = hasher.finish();
        self.clock += 1;
        if !self.entries.contains_key(&key) {
            self.evict();
            let lines = safe::catch(AssertUnwindSafe(|| renderer.render(source, cx))).flatten();
            self.entries.insert(key, Entry { lines, used: 0 });
        }
        let entry = self.entries.get_mut(&key)?;
        entry.used = self.clock;
        entry.lines.clone()
    }

    fn evict(&mut self) {
        if self.entries.len() < CACHE_SIZE {
            return;
        }
        if let Some(&oldest) = self
            .entries
            .iter()
            .min_by_key(|(_, e)| e.used)
            .map(|(k, _)| k)
        {
            self.entries.remove(&oldest);
        }
    }
}

const MAX_OUTPUT: u64 = 4 << 20;

/// Runs a program with the block's source on stdin and shows what it prints. No shell is
/// involved; MAR_LANG, MAR_WIDTH and MAR_ASCII describe the block.
pub struct External {
    name: String,
    program: String,
    args: Vec<String>,
    timeout: Duration,
}

impl External {
    pub fn new(mut argv: Vec<String>, timeout: Duration) -> Option<Self> {
        if argv.is_empty() {
            return None;
        }
        let name = argv.join(" ");
        let program = argv.remove(0);
        Some(External {
            name,
            program,
            args: argv,
            timeout,
        })
    }
}

impl Render for External {
    fn name(&self) -> &str {
        &self.name
    }

    fn render(&self, source: &str, cx: &Context) -> Option<Vec<String>> {
        let mut command = Command::new(&self.program);
        command
            .args(&self.args)
            .env("MAR_LANG", cx.lang)
            .env("MAR_WIDTH", cx.width.to_string())
            .env("MAR_ASCII", if cx.ascii { "1" } else { "0" });
        let output = process::run(command, source.into(), self.timeout, MAX_OUTPUT)?;
        lines(&plain(&String::from_utf8_lossy(&output)))
    }
}

/// Output lines without trailing spaces or surrounding blank lines; `None` if empty.
pub fn lines(text: &str) -> Option<Vec<String>> {
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    let first = lines.iter().position(|l| !l.is_empty())?;
    let last = lines.iter().rposition(|l| !l.is_empty())?;
    Some(lines[first..=last].iter().map(|&l| l.to_owned()).collect())
}

/// Box lines and arrows, as opposed to the labels inside a diagram.
pub fn is_line(c: char) -> bool {
    matches!(c, '\u{2190}'..='\u{21ff}' | '\u{2500}'..='\u{257f}' | '\u{25b2}'..='\u{25c5}')
}

/// Text without terminal escape sequences or control characters other than tab and newline.
fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\x1b' => match chars.next() {
                Some('[') => skip_csi(&mut chars),
                Some(']' | 'P' | 'X' | '^' | '_') => skip_string(&mut chars),
                _ => {}
            },
            '\u{9b}' => skip_csi(&mut chars),
            '\u{90}' | '\u{98}' | '\u{9d}' | '\u{9e}' | '\u{9f}' => skip_string(&mut chars),
            '\n' | '\t' => out.push(c),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

fn skip_csi(chars: &mut impl Iterator<Item = char>) {
    for c in chars.by_ref() {
        if ('\x40'..='\x7e').contains(&c) {
            break;
        }
    }
}

/// Skips an OSC, DCS or similar string up to its terminator (BEL, ST or ESC \).
fn skip_string<I: Iterator<Item = char>>(chars: &mut Peekable<I>) {
    while let Some(c) = chars.next() {
        match c {
            '\x07' | '\u{9c}' => break,
            '\x1b' => {
                chars.next_if_eq(&'\\');
                break;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Instant;

    fn cx(lang: &str) -> Context<'_> {
        Context {
            lang,
            width: 40,
            ascii: false,
        }
    }

    fn external(argv: &[&str], timeout_ms: u64) -> External {
        let argv = argv.iter().map(|s| s.to_string()).collect();
        External::new(argv, Duration::from_millis(timeout_ms)).unwrap()
    }

    struct Panics;

    impl Render for Panics {
        fn name(&self) -> &str {
            "panics"
        }

        fn render(&self, _: &str, _: &Context) -> Option<Vec<String>> {
            panic!("renderer bug")
        }
    }

    struct Counts(std::cell::Cell<usize>);

    impl Render for Counts {
        fn name(&self) -> &str {
            "counts"
        }

        fn render(&self, source: &str, cx: &Context) -> Option<Vec<String>> {
            self.0.set(self.0.get() + 1);
            Some(vec![format!("{source} at {}", cx.width)])
        }
    }

    #[test]
    fn registry_prefers_earlier_entries_and_ignores_case() {
        let mut registry = Registry::default();
        registry.add("Dot", Box::new(Counts(Default::default())));
        registry.add("dot", Box::new(Panics));
        let first = registry.get("DOT").unwrap();
        assert_eq!(first.render("g", &cx("dot")), Some(vec!["g at 40".into()]));
        assert!(registry.get("plantuml").is_none());
    }

    #[cfg(unix)]
    #[test]
    fn configured_commands_override_builtins() {
        let commands = BTreeMap::from([(
            "mermaid".to_owned(),
            config::Command::Line("tr a-z A-Z".into()),
        )]);
        let registry = Registry::new(commands, Duration::from_secs(5)).unwrap();
        let lines = registry
            .get("mermaid")
            .unwrap()
            .render("graph", &cx("mermaid"));
        assert_eq!(lines, Some(vec!["GRAPH".into()]));
    }

    #[test]
    fn empty_command_is_an_error() {
        let commands = BTreeMap::from([("dot".to_owned(), config::Command::Args(vec![]))]);
        assert!(Registry::new(commands, Duration::from_secs(1)).is_err());
    }

    #[test]
    fn cache_reuses_results_per_input() {
        let renderer = Counts(Default::default());
        let mut cache = Cache::default();
        let a = cache.render(&renderer, "x", &cx("t"));
        assert_eq!(cache.render(&renderer, "x", &cx("t")), a);
        assert_eq!(renderer.0.get(), 1);
        let wider = Context {
            width: 80,
            ..cx("t")
        };
        assert_eq!(
            cache.render(&renderer, "x", &wider),
            Some(vec!["x at 80".into()])
        );
        assert_eq!(renderer.0.get(), 2);
    }

    #[test]
    fn cache_keeps_renderers_apart() {
        let mut cache = Cache::default();
        assert!(
            cache
                .render(&Counts(Default::default()), "x", &cx("t"))
                .is_some()
        );
        assert_eq!(cache.render(&Panics, "x", &cx("t")), None);
    }

    #[test]
    fn cache_turns_panics_into_failures() {
        assert_eq!(Cache::default().render(&Panics, "x", &cx("t")), None);
    }

    #[test]
    fn cache_stays_bounded() {
        let renderer = Counts(Default::default());
        let mut cache = Cache::default();
        for i in 0..CACHE_SIZE + 10 {
            cache.render(&renderer, &i.to_string(), &cx("t"));
        }
        assert_eq!(cache.entries.len(), CACHE_SIZE);
    }

    #[cfg(unix)]
    #[test]
    fn external_commands() {
        assert_eq!(
            external(&["tr", "a-z", "A-Z"], 5000).render("abc\n", &cx("t")),
            Some(vec!["ABC".into()])
        );
        assert_eq!(external(&["false"], 5000).render("abc", &cx("t")), None);
        assert_eq!(
            external(&["no-such-program-mar"], 5000).render("abc", &cx("t")),
            None
        );
        let env = external(&["sh", "-c", "echo $MAR_LANG $MAR_WIDTH $MAR_ASCII"], 5000);
        assert_eq!(env.render("", &cx("dot")), Some(vec!["dot 40 0".into()]));
    }

    #[cfg(unix)]
    #[test]
    fn timeouts_stop_spawned_processes() {
        let marker = format!("mar-orphan-{}", std::process::id());
        let script = format!("sleep 30 & echo $! > /tmp/{marker}; wait");
        assert_eq!(
            external(&["sh", "-c", &script], 300).render("", &cx("t")),
            None
        );
        let pid = std::fs::read_to_string(format!("/tmp/{marker}")).unwrap();
        let _ = std::fs::remove_file(format!("/tmp/{marker}"));
        thread::sleep(Duration::from_millis(100));
        assert!(!std::path::Path::new(&format!("/proc/{}", pid.trim())).exists());
    }

    #[cfg(unix)]
    #[test]
    fn slow_commands_time_out() {
        let start = Instant::now();
        assert_eq!(external(&["sleep", "5"], 200).render("", &cx("t")), None);
        assert!(start.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn output_is_plain_text() {
        let text =
            "\x1b[31mred\x1b[0m \x1b]0;title\x07ok\x1b]8;;http://x\x1b\\link\r\n\u{9b}1mb\x07";
        assert_eq!(plain(text), "red oklink\nb");
    }

    #[test]
    fn output_lines_are_trimmed() {
        assert_eq!(
            lines("\n\n  a  \n\nb\n\n"),
            Some(vec!["  a".into(), "".into(), "b".into()])
        );
        assert_eq!(lines(" \n\n"), None);
    }

    #[test]
    fn line_characters() {
        assert!(is_line('─') && is_line('▸') && is_line('→'));
        assert!(!is_line('A') && !is_line('█'));
    }
}
