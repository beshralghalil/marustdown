# marustdown

A fast terminal markdown viewer. It renders GitHub-flavored markdown with real
typography (headings, code boxes with syntax highlighting, tables, alerts, task lists)
in a full-screen pager, and every color, glyph and key can be changed in one TOML file.

```
     ██ Usage
     ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

     Run the server with your config file. See the configuration↗
     section for all options.

     ╭─ rust ──────────────────────────────────────── [y] copy ─╮
     │ 1  fn main() {                                           │
     │ 2      let cfg = Config::load("app.toml")?;              │
     │ 3      server::run(cfg).await                            │
     │ 4  }                                                     │
     ╰──────────────────────────────────────────────────────────╯

     ▌ 󰋽 NOTE
     ▌ Requires Rust 1.80 or newer.

     ┌──────────┬─────────┬──────────────────────────┐
     │ Flag     │ Default │ Description              │
     ├──────────┼─────────┼──────────────────────────┤
     │ --port   │ 8080    │ Port to listen on        │
     └──────────┴─────────┴──────────────────────────┘

     ✔ Write docs
     ☐ Add tests

 ln 12/40 │ Usage › Install        / search  n next  x toggle  o outline  q quit
```

## Features

- **Pager** with a cursor line, search, heading jumps and a breadcrumb of the current section.
- **Task lists you can check off.** Toggling a task writes `[ ]` ↔ `[x]` back to the file.
- **Keyboard link following.** `f` tags every visible link, and typing a tag opens it
  (in uppercase, it copies the URL instead). Tab and Shift-Tab step through links, and Enter
  opens the selected one. `#anchors` jump within the page, linked `.md` files open in
  the viewer with a back history, and everything else goes to `xdg-open`.
- **Outline picker.** Filter the headings by typing, then jump to one.
- **Edit in `$EDITOR`** at the cursor's source line, and see the result when you return.
- **Copy code blocks** to the system clipboard with OSC 52, which also works over SSH.
- **Syntax highlighting** for Rust, Python, JS/TS, Go, C/C++, shell, JSON, TOML/YAML, SQL
  and Lua, with no highlighting library.
- **Clickable links** (OSC 8) in terminals that support them.
- **Cat mode**: styled output to stdout, used automatically when the output is piped.
- **Fully configurable**: themes, per-element styles, glyphs, layout and key bindings.
- **Fast and small**: memory-mapped input and a flat three-buffer layout with no
  per-line allocations. The release binary is about 1.4 MB.

## Install

Requires Rust 1.88 or newer.

```sh
cargo install --path .
```

This installs the `mar` binary. Alert icons use [Nerd Font](https://www.nerdfonts.com/)
glyphs; without one, run with `--no-icons` or set `icons = false`.

## Usage

```sh
mar README.md            # page a file
mar README.md --cat      # print styled output and exit
cat notes.md | mar       # read stdin (`mar -` works too)
mar doc.md | less -R     # piping switches to cat mode automatically
```

| Option | |
|---|---|
| `--cat` | Print and exit instead of paging |
| `--width <N>` | Maximum content width (default 90) |
| `--theme <NAME>` | `dark`, `light`, `ansi`, or a custom theme file |
| `--no-color` | Plain text; `NO_COLOR` is honored too |
| `--no-icons` | ASCII glyphs instead of Unicode and Nerd Font icons |
| `--config <PATH>` | Use this config file instead of the default location |

### Keys

| Key | Action |
|---|---|
| `j` `k` / `↓` `↑` | Move the cursor |
| `Space` / `b`, `PgDn` / `PgUp` | Page down / up |
| `d` / `u` | Half page down / up |
| `g` / `G` | Top / bottom |
| `]]` / `[[` | Next / previous heading |
| `/` then `Enter`, `n` / `N` | Search (smart case), next / previous match |
| `f` then a tag | Open that link; type the tag in uppercase to copy its URL |
| `Tab` / `Shift-Tab` | Select the next / previous link |
| `Enter` | Open the selected link, or toggle the task under the cursor |
| `Backspace` | Go back to the previous file after following a link |
| `x` | Toggle the task under the cursor (saved to the file) |
| `o` | Outline: type to filter, `↑` `↓` to pick, `Enter` to jump, `Esc` to close |
| `e` | Open `$VISUAL` / `$EDITOR` at the cursor line, then reload |
| `y` | Copy the code block at the cursor |
| `q` / `Esc` | Quit (`Ctrl-C` always quits) |

Every binding except `Ctrl-C` can be changed, see [Keys](#key-bindings).

## Configuration

The config file lives at `$XDG_CONFIG_HOME/marustdown/config.toml`, which is usually
`~/.config/marustdown/config.toml`. You don't have to create one. When you do, include
only what you want to change: anything missing keeps its default, and tables merge key by
key. All defaults, with comments, are in [`assets/config.toml`](assets/config.toml).

```toml
theme = "light"

[layout]
width = 100
center = false

[styles.h1]
fg = "#ff8800"     # h1 stays bold; only the color changes

[glyphs]
bullets = ["-", "*"]

[keys]
toggle = ["space"]
page_down = ["f", "pagedown"]
```

Unknown keys and invalid values are reported with their name, so typos don't pass silently.

### Themes

`theme` picks the color palette: `dark` (the default), `light`, or `ansi`. `ansi` uses
your terminal's own 16 colors, so it follows whatever terminal theme you already have.

To make your own theme, create `~/.config/marustdown/themes/<name>.toml` and set
`theme = "<name>"`. A theme file needs a `[colors]` table, and it may also override
`[styles]`, `[glyphs]` or any other section:

```toml
[colors]
accent = "#e06c75"
secondary = "#c678dd"
subtle = "#abb2bf"
overlay = "#7f848e"
muted = "#5c6370"
surface = "#2c313a"
highlight = "#3e4451"
blue = "#61afef"
green = "#98c379"
yellow = "#e5c07b"
orange = "#d19a66"
red = "#e06c75"
purple = "#c678dd"
teal = "#56b6c2"
```

### Colors

A color is written as one of:

- `"#rrggbb"` or `"#rgb"` for true color,
- `0` to `255` for your terminal's palette (0–15 follow the terminal theme),
- `"default"` for the terminal's own foreground or background,
- the name of an entry in `[colors]`. You can add your own names there too.

### Styles

Each entry in `[styles]` is an inline table that accepts `fg`, `bg`, `bold`, `dim`,
`italic`, `underline`, `strike` and `reverse`.

| Group | Styles |
|---|---|
| Text | `text`, `strong`, `emphasis`, `strike`, `code`, `link`, `link_icon`, `image` |
| Headings | `h1` … `h6` (a heading's rule line uses its own style) |
| Quotes and alerts | `quote`, `quote_bar`, `alert_note`, `alert_tip`, `alert_important`, `alert_warning`, `alert_caution` |
| Lists | `bullet`, `number`, `task_done`, `task_todo`, `rule` |
| Code blocks | `code_block`, `code_border`, `code_label`, `line_number` |
| Syntax | `syntax_keyword`, `syntax_string`, `syntax_number`, `syntax_comment`, `syntax_type` |
| Tables | `table_border`, `table_header` |
| Pager | `cursor`, `status`, `search`, `outline_level`, `hint`, `link_selected` |

### Glyphs

`[glyphs]` controls every character the viewer draws: heading markers and rules, bullets
by depth, task boxes, the quote bar, the link and image markers, the code box and table
border sets, alert titles, the ellipsis for cut-off code lines, and the breadcrumb and
status separators. Set any of them to `""` to hide it. `icons = false` or `--no-icons`
switches to an ASCII set, and any glyphs you set yourself still apply on top of it.

### Layout

| Key | Default | |
|---|---|---|
| `width` | `90` | Maximum content width in columns |
| `margin` | `2` | Minimum free columns on each side |
| `center` | `true` | Center the content; otherwise indent by `margin` |
| `line_numbers` | `true` | Number code block lines |
| `tab_width` | `4` | Tab stops in code blocks |
| `scroll_off` | `3` | Lines kept visible around the cursor |
| `color` | `true` | `false` is the same as `--no-color` |
| `icons` | `true` | `false` is the same as `--no-icons` |
| `status_bar` | `true` | Show the pager's status bar |

### Key bindings

Each action in `[keys]` takes a list of keys. A key is one of:

- a character: `"j"`, `"G"`, `"/"`,
- a sequence of up to four characters: `"]]"`, `"gg"`,
- a named key: `up`, `down`, `left`, `right`, `pageup`, `pagedown`, `home`, `end`,
  `enter`, `esc`, `tab`, `backtab`, `backspace`, `delete`, `space`,
- a modifier combination: `"ctrl-d"`, `"alt-x"`, `"ctrl-pagedown"`.

The actions are `down`, `up`, `page_down`, `page_up`, `half_down`, `half_up`, `top`,
`bottom`, `next_heading`, `prev_heading`, `search`, `next_match`, `prev_match`, `toggle`,
`next_link`, `prev_link`, `open`, `hints`, `back`, `copy`, `outline`, `edit` and `quit`. The status bar hints update to match your bindings.

## Notes

- Links to local markdown files are resolved against the current file's directory, and
  a `#fragment` jumps to the matching heading using GitHub's anchor rules. External links
  open with `xdg-open` (`open` on macOS).
- Copying needs a terminal that supports OSC 52. Under tmux, also set `set -g set-clipboard on`.
- Toggling a task checks that the file still has a task box at that spot before writing,
  so a file edited behind the viewer's back won't be corrupted.
- Input from stdin can be viewed and its tasks toggled, but those changes aren't saved.

## Development

```sh
cargo test       # unit tests for layout, wrapping, tables, highlighting, config, keys
cargo clippy --all-targets
```

The design is in [plan.md](plan.md). In short, `source` memory-maps the input, `layout`
walks the pulldown-cmark events into a `Document` (flat text, style runs and line ends,
with no escape codes), and `render` and `pager` turn it into terminal output without
knowing anything about markdown.

## License

[MIT](LICENSE)
