use std::path::PathBuf;

use clap::{Parser, ValueEnum};

const KEYS: &str = "\
Keys:
  j/k, Up/Down     move the cursor
  Space/b          page down / up
  ]] / [[          next / previous heading
  /  n  N          search, next / previous match
  x                toggle the task under the cursor (saved to the file)
  f                tag visible links; type a tag to open it, uppercase to copy the URL
  Tab / Enter      select / open a link
  Backspace        back to the previous file
  o                outline
  y                copy the code block
  e                edit in $VISUAL or $EDITOR
  q                quit

Configuration: $XDG_CONFIG_HOME/marustdown/config.toml (usually ~/.config/marustdown/config.toml).
Reference: https://github.com/beshralghalil/marustdown/blob/main/docs/configuration.md";

/// Optional Cargo features, and whether this build has them.
const FEATURES: [(&str, bool); 3] = [
    ("highlight", cfg!(feature = "highlight")),
    ("math", cfg!(feature = "math")),
    ("mermaid", cfg!(feature = "mermaid")),
];

/// The version, followed by the optional features compiled in.
fn version() -> String {
    let features: Vec<&str> = FEATURES.iter().filter(|f| f.1).map(|f| f.0).collect();
    let features = if features.is_empty() {
        "no optional features".to_owned()
    } else {
        features.join(", ")
    };
    format!("{} ({features})", env!("CARGO_PKG_VERSION"))
}

/// A fast, configurable terminal markdown viewer.
#[derive(Parser)]
#[command(
    name = "mar",
    version = version(),
    long_about = "Render GitHub-flavored markdown in a full-screen pager, with syntax-highlighted \
                  code, task lists you can check off, keyboard link following and an outline. \
                  When stdout isn't a terminal, the rendered document is printed instead.",
    after_long_help = KEYS
)]
pub struct Args {
    /// Markdown file to view; `-` or nothing reads stdin
    pub file: Option<PathBuf>,
    /// Print the rendered document and exit instead of paging (automatic when stdout isn't a terminal)
    #[arg(long)]
    pub cat: bool,
    /// When to style printed output: `auto` styles only when writing to a terminal
    #[arg(long, value_enum, value_name = "WHEN", default_value_t = ColorMode::Auto)]
    pub color: ColorMode,
    /// Maximum content width in columns
    #[arg(long, value_name = "N")]
    pub width: Option<usize>,
    /// Color preset: dark, light, ansi, or a theme file name
    #[arg(long, value_name = "NAME")]
    pub theme: Option<String>,
    /// Same as `--color never` (NO_COLOR is honored too)
    #[arg(long)]
    pub no_color: bool,
    /// Reload the file when it changes on disk (also `watch = true` in the config)
    #[arg(long, overrides_with = "no_watch")]
    pub watch: bool,
    /// Don't watch the file, even if the config enables it
    #[arg(long, overrides_with = "watch")]
    pub no_watch: bool,
    /// Use ASCII glyphs instead of icons
    #[arg(long)]
    pub no_icons: bool,
    /// Config file to use instead of $XDG_CONFIG_HOME/marustdown/config.toml
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ColorMode {
    Auto,
    Always,
    Never,
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use clap::CommandFactory;
    use clap_complete::{Shell, generate};

    use super::*;

    /// Compares a committed generated file, or rewrites it when UPDATE_GENERATED is set.
    fn check(path: &str, contents: &[u8]) {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
        if std::env::var_os("UPDATE_GENERATED").is_some() {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, contents).unwrap();
            return;
        }
        let current = fs::read(&path).unwrap_or_default();
        assert!(
            current == contents,
            "{} is out of date; run `UPDATE_GENERATED=1 cargo test`",
            path.display()
        );
    }

    #[test]
    fn man_page_and_completions_are_current() {
        let mut man = Vec::new();
        // Without the version, release bumps don't make the committed page stale.
        clap_mangen::Man::new(Args::command().version(None))
            .source("marustdown")
            .render(&mut man)
            .unwrap();
        check("assets/man/mar.1", &man);
        for (shell, file) in [
            (Shell::Bash, "mar.bash"),
            (Shell::Zsh, "_mar"),
            (Shell::Fish, "mar.fish"),
        ] {
            let mut out = Vec::new();
            generate(shell, &mut Args::command(), "mar", &mut out);
            check(&format!("assets/completions/{file}"), &out);
        }
    }

    #[test]
    fn version_lists_features() {
        let version = version();
        assert!(version.starts_with(env!("CARGO_PKG_VERSION")));
        assert_eq!(version.contains("mermaid"), cfg!(feature = "mermaid"));
    }

    #[test]
    fn cli_is_valid() {
        Args::command().debug_assert();
    }
}
