# Stress fixtures

Markdown that tries to break marustdown. `tests/stress.rs` renders every file through the
`mar` binary on each `cargo test`.

| File | Covers |
|---|---|
| `wide.md` | Horizontal scrolling: long code lines, wide characters at the cut edges, tabs, the 4,096-column cap, wide math, a wide diagram, wide blocks in quotes and lists |
| `mermaid.md` | Every Mermaid diagram type, a large graph, broken and empty diagrams, a crash in the renderer |
| `math.md` | Inline and display LaTeX, matrices and cases, math in headings, tables, lists and quotes, dollar amounts, broken LaTeX |
| `nasty.md` | Terminal escape injection, a 600-character word, deep nesting, Unicode edge cases, many links, odd tables and headings, HTML, an unclosed fence |
| `numbered.md` | A 1,200-item ordered list |
| `tasks.md` | Task toggling; toggling changes the file |
| `links.md` | Anchors, links to other files, missing targets, external links |
| `crlf-bom.md`, `empty.md`, `whitespace.md`, `invalid-utf8.md` | Line endings, byte order mark, empty input, broken encoding |

Try one in the pager with `cargo run --release -- tests/stress/wide.md`.

`python3 generate.py` rewrites the fixtures (after toggling tasks, for example) and also
writes `big.md`, a 1.3 MB mix of everything for speed tests. `big.md` is ignored by git.
