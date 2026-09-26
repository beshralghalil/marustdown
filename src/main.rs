mod config;
mod doc;
mod highlight;
mod keys;
mod layout;
mod links;
mod pager;
mod render;
mod search;
mod source;
mod table;
mod theme;
mod wrap;

use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::error::ErrorKind;
use clap::{CommandFactory, Parser, ValueEnum};

use doc::Document;
use keys::Keymap;
use source::Source;
use theme::Theme;

/// A terminal markdown viewer.
#[derive(Parser)]
#[command(version)]
struct Args {
    /// Markdown file to view; `-` or nothing reads stdin
    file: Option<PathBuf>,
    /// Print the rendered document and exit instead of paging (automatic when stdout isn't a terminal)
    #[arg(long)]
    cat: bool,
    /// When to style printed output: `auto` styles only when writing to a terminal
    #[arg(long, value_enum, value_name = "WHEN", default_value_t = ColorMode::Auto)]
    color: ColorMode,
    /// Maximum content width in columns
    #[arg(long, value_name = "N")]
    width: Option<usize>,
    /// Color preset: dark, light, ansi, or a theme file name
    #[arg(long, value_name = "NAME")]
    theme: Option<String>,
    /// Same as `--color never` (NO_COLOR is honored too)
    #[arg(long)]
    no_color: bool,
    /// Use ASCII glyphs instead of icons
    #[arg(long)]
    no_icons: bool,
    /// Config file to use instead of $XDG_CONFIG_HOME/marustdown/config.toml
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum ColorMode {
    Auto,
    Always,
    Never,
}

fn main() -> ExitCode {
    match run(Args::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("mar: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Args) -> Result<(), String> {
    let mut cfg = config::load(args.config.as_deref(), args.theme.as_deref(), args.no_icons)?;
    if let Some(width) = args.width {
        cfg.layout.width = width;
    }
    let color = if args.no_color {
        ColorMode::Never
    } else {
        args.color
    };
    if color == ColorMode::Never || std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()) {
        cfg.layout.color = false;
    }
    let terminal = io::stdout().is_terminal();
    let paged = !args.cat && terminal;
    let plain = match color {
        ColorMode::Auto => !terminal,
        ColorMode::Always => false,
        ColorMode::Never => true,
    };
    if !paged {
        cfg.glyphs.code_copy.clear();
    }
    let theme = Theme::new(cfg)?;
    let keys = Keymap::new(&theme.keys)?;

    let path = args.file.filter(|p| p.as_os_str() != "-");
    let source = match &path {
        Some(p) => Source::open(p).map_err(|e| format!("{}: {e}", p.display()))?,
        None if io::stdin().is_terminal() => Args::command()
            .error(
                ErrorKind::MissingRequiredArgument,
                "no input: pass a file or pipe markdown to stdin",
            )
            .exit(),
        None => Source::stdin().map_err(|e| format!("stdin: {e}"))?,
    };

    let result = if paged {
        pager::run(source, path, &theme, keys)
    } else {
        cat(&source, &theme, plain)
    };
    match result {
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        r => r.map_err(|e| e.to_string()),
    }
}

fn cat(source: &Source, theme: &Theme, plain: bool) -> io::Result<()> {
    let (width, margin) = match crossterm::terminal::size() {
        Ok((cols, _)) if io::stdout().is_terminal() => theme.layout.fit(cols as usize),
        _ => (theme.layout.width, 0),
    };
    let mut doc = Document::new();
    doc.layout(source.text(), width, theme);
    render::cat(&doc, theme, margin, plain, &mut io::stdout().lock())
}
