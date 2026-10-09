# Configuration

## File

- Path: `$XDG_CONFIG_HOME/marustdown/config.toml`, usually `~/.config/marustdown/config.toml`.
- Use another file with `mar --config <path>`.
- Every key is optional. Missing keys keep their defaults.
- Tables merge key by key: `[styles.h1] fg = "red"` changes only the color, so h1 stays bold.
- Unknown keys are errors.
- The full default file is [`assets/config.toml`](../assets/config.toml).

Example:

```toml
theme = "light"

[layout]
width = 100

[styles.h1]
fg = "#ff8800"

[keys]
toggle = ["space"]
```

## Theme

```toml
theme = "ansi"
```

| Value | Colors |
|---|---|
| `ansi` | Your terminal's 16 colors (default) |
| `dark` | Catppuccin Mocha |
| `light` | Catppuccin Latte |
| `<name>` | `~/.config/marustdown/themes/<name>.toml` |

A theme file holds a `[colors]` table and may override any other table.
`--theme <name>` overrides the config.

## `[layout]`

| Key | Default | Meaning |
|---|---|---|
| `width` | `90` | Maximum content width, in columns |
| `margin` | `2` | Minimum empty columns on each side |
| `center` | `true` | Center the content (otherwise indent by `margin`) |
| `line_numbers` | `true` | Number the lines of code blocks |
| `tab_width` | `4` | Tab stop width in code blocks, 1–16 |
| `scroll_off` | `3` | Lines kept visible above and below the cursor |
| `scroll_duration` | `120` | Milliseconds a page, heading or search jump glides (0–1000); `0` jumps instantly |
| `color` | `true` | `false` turns colors off; the cursor line is then shown in reverse video |
| `icons` | `true` | `false` switches to ASCII glyphs |
| `status_bar` | `true` | Show the status bar |
| `math` | `true` | Render `$…$` and `$$…$$` LaTeX as Unicode; `false` shows the source (needs the `math` feature) |
| `watch` | `false` | Reload the file when it changes on disk; `--watch` / `--no-watch` override it |
| `diagrams` | `true` | Draw ```` ```mermaid ```` and [`[renderers]`](#renderers) blocks; `false` shows their source |
| `renderer_timeout` | `3000` | Milliseconds (1–60000) a `[renderers]` command may take before its block shows as code |

## `[colors]`

Named colors, used by name in `[styles]`. The built-in themes define `accent`,
`secondary`, `subtle`, `overlay`, `muted`, `surface`, `highlight`, `blue`, `green`,
`yellow`, `orange`, `red`, `purple` and `teal`. You can add your own names.

| Value | Example |
|---|---|
| Hex | `"#89b4fa"`, `"#fff"` |
| Terminal palette | `4` (0–255; 0–15 follow the terminal theme) |
| Terminal default | `"default"` |

## `[styles]`

Each style is an inline table:

| Key | Value |
|---|---|
| `fg`, `bg` | A color name from `[colors]`, a color value, or `"default"` |
| `bold`, `dim`, `italic`, `underline`, `strike`, `reverse` | `true` / `false` |

```toml
[styles]
code = { fg = "orange", bg = "surface" }
```

| Style | Element |
|---|---|
| `text` | Body text |
| `h1` … `h6` | Headings and their rule lines |
| `strong`, `emphasis`, `strike` | `**bold**`, `*italic*`, `~~strike~~` |
| `code` | Inline code |
| `math` | Inline and display math |
| `diagram`, `diagram_border` | Labels in Mermaid and `[renderers]` blocks, and their boxes, lines and arrows |
| `link`, `link_icon` | Link text and the `↗` after it |
| `image` | Image alt text |
| `quote`, `quote_bar` | Blockquote text and bar |
| `alert_note`, `alert_tip`, `alert_important`, `alert_warning`, `alert_caution` | Alert bar and title |
| `bullet`, `number` | List markers |
| `task_done`, `task_todo` | Task boxes |
| `rule` | Horizontal rules |
| `code_block`, `code_border`, `code_label`, `line_number` | Code block background, box, language and copy label, line numbers |
| `syntax_keyword`, `syntax_string`, `syntax_number`, `syntax_comment`, `syntax_type`, `syntax_function`, `syntax_constant`, `syntax_operator`, `syntax_tag`, `syntax_attribute`, `syntax_inserted`, `syntax_deleted` | Highlighted code tokens (needs the `highlight` feature) |
| `table_border`, `table_header` | Table borders and header row |
| `cursor` | Cursor line |
| `status` | Status bar |
| `search` | Search matches |
| `outline_level` | Line numbers in the outline |
| `hint` | Link tags shown by `f` |
| `link_selected` | Link selected with Tab |
| `overflow` | ‹ › markers where a wide block continues off screen |

## `[glyphs]`

An empty string hides the element. `icons = false` or `--no-icons` switches to an
ASCII set; glyphs you set yourself still apply on top of it.

| Key | Default | Element |
|---|---|---|
| `heading` | `["██", "▌", "", "", "", ""]` | Marker before h1 … h6 |
| `heading_rule` | `["━", "─", "", "", "", ""]` | Rule under h1 … h6 |
| `bullets` | `["•", "◦", "▪"]` | Bullets by nesting depth, repeating |
| `task_done`, `task_todo` | `"✔"`, `"☐"` | Task boxes |
| `quote` | `"▌"` | Quote and alert bar |
| `link` | `"↗"` | After link text |
| `image` | `"🖼"` | Before image alt text |
| `rule` | `"─"` | Horizontal rule |
| `ellipsis` | `"…"` | Where a wide block is cut off in `--cat` output |
| `overflow_left`, `overflow_right` | `"‹"`, `"›"` | A wide block continues to the left / right in the pager |
| `code_copy` | `"[y] copy"` | Copy hint on code blocks |
| `code_box` | `["╭", "─", "╮", "│", "╰", "╯"]` | Top-left, horizontal, top-right, vertical, bottom-left, bottom-right |
| `table_box` | `["┌", "┬", "┐", "├", "┼", "┤", "└", "┴", "┘", "│", "─"]` | Top, middle and bottom rows (left, cross, right), then vertical, horizontal |
| `alert_note` … `alert_caution` | Nerd Font icon + `NOTE` … | Alert titles |
| `breadcrumb` | `" › "` | Between headings in the status bar |
| `separator` | `" │ "` | Between status bar sections |

## `[renderers]`

Programs that draw fenced code blocks, by fence language:

```toml
[renderers]
dot = "graph-easy --as=boxart"
plantuml = ["env", "PLANTUML_SECURITY_PROFILE=SANDBOX", "plantuml", "-tutxt", "-pipe"]
```

- The block's source goes to the program's stdin, and its output replaces the code box.
- A command is a string split on spaces, or a list of arguments. No shell is used.
- The program gets `MAR_LANG` (lowercase), `MAR_WIDTH` (columns available) and `MAR_ASCII` (`1` with icons off).
- The output is shown as plain text in the `diagram` and `diagram_border` styles.
- If the program fails, prints nothing or runs past `renderer_timeout`, the code is shown,
  and the program and anything it started are stopped.
- Results are cached: a block runs again only when its source or the width changes.
- A renderer for `mermaid` replaces the built-in one.
- Renderers also run on documents you didn't write, so only configure programs that are safe
  on untrusted input. PlantUML's sandbox profile above, for example, blocks file and URL includes.

## `[keys]`

Each action takes a list of keys:

| Form | Example |
|---|---|
| Character | `"j"`, `"G"`, `"/"` |
| Sequence (up to 4) | `"]]"` |
| Named key | `up` `down` `left` `right` `pageup` `pagedown` `home` `end` `enter` `esc` `tab` `backtab` `backspace` `delete` `space` |
| Modifier | `"ctrl-d"`, `"alt-x"` |

| Action | Default | Does |
|---|---|---|
| `down`, `up` | `j` `down`, `k` `up` | Move the cursor |
| `page_down`, `page_up` | `space` `pagedown`, `b` `pageup` | Page |
| `half_down`, `half_up` | `d` `ctrl-d`, `u` `ctrl-u` | Half page |
| `top`, `bottom` | `g` `home`, `G` `end` | Start / end |
| `next_heading`, `prev_heading` | `]]`, `[[` | Heading jumps |
| `search`, `next_match`, `prev_match` | `/`, `n`, `N` | Search |
| `toggle` | `x` | Toggle the task under the cursor |
| `next_link`, `prev_link` | `tab`, `backtab` | Select a link |
| `open` | `enter` | Open the selected link, or toggle the task |
| `hints` | `f` | Tag visible links; type a tag to open it, uppercase to copy the URL |
| `back` | `backspace` | Previous file |
| `copy` | `y` | Copy the code block |
| `outline` | `o` | Outline |
| `edit` | `e` | Open `$VISUAL` / `$EDITOR` at the cursor line |
| `scroll_left`, `scroll_right` | `h` `left`, `l` `right` | Scroll the wide block under the cursor by 8 columns |
| `scroll_home`, `scroll_end` | `H`, `L` | Scroll it to the start / end |
| `quit` | `q` `esc` | Quit |

Ctrl-C always quits.
