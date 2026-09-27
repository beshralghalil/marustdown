#!/usr/bin/env python3
"""Writes the stress-test fixtures next to this script. big.md is generated only, never committed."""
from pathlib import Path

HERE = Path(__file__).parent


def write(name, text):
    path = HERE / name
    if isinstance(text, bytes):
        path.write_bytes(text)
    else:
        path.write_text(text, encoding="utf-8", newline="")
    print(f"{name:22} {path.stat().st_size:>12,} bytes")


LONG = "this line is much longer than any sane content width " * 4

wide = f"""# Horizontal scrolling

Put the cursor on a block and press `l` / `h` (or → / ←); `L` / `H` jump to the ends.
Text around the blocks must not move.

## Long code line among short ones

```rust
fn short() {{}}
fn main() {{ let s = "{LONG}"; println!("{{s}}"); }}
fn also_short() {{}}
```

## Wide characters at the cut edges

Scroll one step at a time: columns must stay aligned, CJK and emoji cut in half become spaces.

```text
日本語のテキスト日本語のテキスト日本語のテキスト日本語のテキスト日本語のテキスト日本語のテキスト
🚀✨🎉🔥💡🚀✨🎉🔥💡🚀✨🎉🔥💡🚀✨🎉🔥💡🚀✨🎉🔥💡🚀✨🎉🔥💡🚀✨🎉🔥💡🚀✨🎉🔥💡🚀✨🎉🔥💡
mixed ascii 日本 ascii 🚀 ascii 日本 ascii 🚀 ascii 日本 ascii 🚀 ascii 日本 ascii 🚀 ascii 日本 ascii 🚀 end
```

## Tabs

```go
func main() {{
\tif true {{
\t\tfmt.Println("{LONG}")
\t}}
}}
```

## One line past the 4096-column cap

```text
{"x" * 5000}END-SHOULD-NOT-BE-VISIBLE
```

## Minified JSON

```json
{{"name":"marustdown","features":["pager","search","tasks","math","diagrams","links","outline","edit"],"nested":{{"a":{{"b":{{"c":{{"d":[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20]}}}}}}}},"long":"{LONG}"}}
```

## Three-digit line numbers, long line deep inside

```python
""" + "\n".join(
    f"x_{i} = {i}" if i != 77 else f"line_77 = '{LONG}'" for i in range(1, 131)
) + f"""
```

## Wide display math

$$
\\sum_{{i=1}}^{{n}} a_i + \\sum_{{j=1}}^{{m}} b_j + \\sum_{{k=1}}^{{p}} c_k + \\int_0^\\infty f(x)\\,dx + \\int_0^\\infty g(x)\\,dx + \\int_0^\\infty h(x)\\,dx + \\alpha + \\beta + \\gamma + \\delta
$$

$$
M = \\begin{{pmatrix}} 1 & 2 & 3 & 4 & 5 & 6 & 7 & 8 & 9 & 10 & 11 & 12 & 13 & 14 & 15 & 16 & 17 & 18 & 19 & 20 \\\\ 21 & 22 & 23 & 24 & 25 & 26 & 27 & 28 & 29 & 30 & 31 & 32 & 33 & 34 & 35 & 36 & 37 & 38 & 39 & 40 \\end{{pmatrix}}
$$

## Wide diagram

```mermaid
graph LR
    A[Collect requirements] --> B[Design the architecture] --> C[Implement features] --> D[Write tests] --> E[Review code] --> F[Release] --> G[Monitor production]
```

## Wide code inside a quote and a list

> The quote bar must stay while the code scrolls:
>
> ```sh
> echo "{LONG}"
> ```

- A list item with a wide block:

  ```sh
  curl --proto '=https' --tlsv1.2 -LsSf https://example.com/some/very/long/path/that/keeps/going/and/going/installer.sh | sh -s -- --yes --verbose
  ```

## A table with many columns (wraps, doesn't scroll)

| {" | ".join(f"c{i}" for i in range(30))} |
|{"|".join("---" for _ in range(30))}|
| {" | ".join(f"value {i}" for i in range(30))} |

Last paragraph.
"""

mermaid = """# Mermaid

## Flowchart top-down

```mermaid
graph TD
    A[Start] --> B{Is it working?}
    B -->|Yes| C[Ship it]
    B -->|No| D[Debug]
    D --> B
```

## Flowchart with subgraphs and edge styles

```mermaid
flowchart LR
    subgraph Frontend
        UI[Web UI] --> API
    end
    subgraph Backend
        API --> DB[(Database)]
        API -.-> Cache((Cache))
        API ==> Queue>Queue]
    end
```

## Sequence with notes and loops

```mermaid
sequenceDiagram
    participant C as Client
    participant S as Server
    C->>S: GET /items
    Note over S: validate token
    loop every page
        S-->>C: 200 page
    end
    C-xS: cancel
```

## State

```mermaid
stateDiagram-v2
    [*] --> Idle
    Idle --> Running: start
    Running --> Idle: stop
    Running --> Failed: error
    Failed --> [*]
```

## Class (ASCII names)

```mermaid
classDiagram
    Animal <|-- Dog
    Animal <|-- Cat
    class Animal {
        +String name
        +speak()
    }
```

## Class with a non-ASCII name (crashes mermaid-text; must show the source)

```mermaid
classDiagram
    class Café
    Café <|-- Bistro
```

## Pie

```mermaid
pie title Pets adopted
    "Dogs" : 386
    "Cats" : 85
    "Rats" : 15
```

## Gantt

```mermaid
gantt
    title Release plan
    dateFormat YYYY-MM-DD
    section Build
    Math     :a1, 2026-01-01, 7d
    Diagrams :after a1, 10d
    section Ship
    Release  :2026-01-20, 2d
```

## Entity relationship

```mermaid
erDiagram
    CUSTOMER ||--o{ ORDER : places
    ORDER ||--|{ LINE_ITEM : contains
```

## Journey, mindmap, timeline, git graph

```mermaid
journey
    title Reading docs
    section Open
      Find file: 5: Me
      Read it: 3: Me
```

```mermaid
mindmap
  root((marustdown))
    Pager
    Math
    Diagrams
```

```mermaid
timeline
    title History
    2024 : idea
    2025 : prototype
    2026 : release
```

```mermaid
gitGraph
    commit
    branch feature
    commit
    checkout main
    merge feature
```

## Broken and odd input (all must show the source, no crash)

```mermaid
graph TD
    A[unclosed --> B{{{
```

```mermaid
notADiagramType
    whatever
```

```mermaid
```

```mermaid
%% only a comment
```

```MERMAID
graph LR
    upper --> case
```

## Big flowchart

```mermaid
graph TD
""" + "\n".join(f"    N{i}[Node {i}] --> N{i + 1}[Node {i + 1}]" for i in range(40)) + """
""" + "\n".join(f"    N{i} --> N{(i * 7) % 40}" for i in range(0, 40, 3)) + """
```
"""

math = r"""# Math

Inline: $x^2 + y_i$, $\frac{a}{b}$, $\sqrt{x^2+1}$, $\alpha\beta\gamma$, $x \in \mathbb{R}$,
$\sum_{i=1}^{n} i$, $\int_0^1 x\,dx$, $\lim_{x \to 0} \frac{\sin x}{x}$, $\vec{v} \cdot \hat{n}$.

Money is not math: it costs $5 and $10, or \$20 escaped. A lone $ sign.

## Display

$$
e^{i\pi} + 1 = 0
$$

$$
f(x) = \begin{cases} x^2 & x \geq 0 \\ -x & \text{otherwise} \end{cases}
$$

$$
\begin{bmatrix} 1 & 0 & 0 \\ 0 & 1 & 0 \\ 0 & 0 & 1 \end{bmatrix} \cdot v = v
$$

$$
A = \begin{pmatrix} a & b \\ c & d \end{pmatrix}, \quad B = \begin{vmatrix} 1 & 2 \\ 3 & 4 \end{vmatrix}
$$

Text before $$\frac{1}{2}$$ and text after, in one paragraph.

## Math in odd places

### Heading with $E = mc^2$ and $$\pi$$

| Formula | Value |
|---------|-------|
| $\pi$ | $3.14$ |
| $$\sqrt{2}$$ | 1.41 |

- list item $a^2 + b^2 = c^2$
  $$
  \nabla \cdot E = \frac{\rho}{\varepsilon_0}
  $$

> quote $$\oint_C F \cdot dr$$ inside

## Broken LaTeX (must not crash)

$\frac{a}{$ and $x^$ and $\sqrt[$ and $\begin{pmatrix} a & b$ and $}}}}$ and $\unknowncommand{x}$.

$$
\begin{cases} 1 & x
$$

$$
$$

$$ $ $$

$$\\\\\\\\$$
"""

esc = "\x1b"
nasty = (
    "# Hostile input\n\n"
    "## Terminal escape injection (must render as harmless replacement characters)\n\n"
    f"Red text attempt: {esc}[31mRED{esc}[0m, clear screen {esc}[2J, cursor move {esc}[10;10H.\n\n"
    f"Clipboard write attempt: {esc}]52;c;cGF3bmVk\x07 and title change {esc}]0;owned\x07.\n\n"
    "Bell \x07, backspace \x08\x08\x08, carriage return \rin the middle, form feed \x0c, NUL-ish \x01.\n\n"
    f"C1 control CSI \u009b31m and OSC \u009d0;x\u009c.\n\n"
    f"[link with escape in url](https://example.com/{esc}]52;c;eA==\x07)\n\n"
    f"```sh\necho '{esc}[31mred inside code{esc}[0m'\n```\n\n"
    "## Very long word\n\n" + "A" * 600 + "\n\n"
    "## Deep nesting\n\n" + "".join("> " * i + f"quote level {i}\n" for i in range(1, 41)) + "\n"
    + "".join("  " * i + f"- list level {i}\n" for i in range(30)) + "\n"
    "## Unicode edge cases\n\n"
    "Zero width​space, ZWJ family 👨‍👩‍👧‍👦, flags 🇸🇾🇩🇪, combining é vs é,\n"
    "RTL: مرحبا بالعالم and שלום עולם, fullwidth ＡＢＣ１２３, math 𝕏𝔸𝕄, box ─│┌┐.\n\n"
    "## Many links (hint tags go to two letters)\n\n"
    + " ".join(f"[link {i}](https://example.com/{i})" for i in range(60)) + "\n\n"
    "## Odd tables\n\n"
    "| a | b | c |\n|---|---|---|\n| only one cell |\n| 1 | 2 | 3 | 4 | 5 |\n|  |  |  |\n\n"
    "| x |\n|---|\n\n"
    "## Empty and odd headings\n\n#\n\n##\n\n###### deep\n\n# `code only`\n\n# **bold** *em* ~~strike~~\n\n"
    "Setext\n===\n\n"
    "## HTML\n\n<div>\n<script>alert(1)</script>\n</div>\n\n<img src=x onerror=alert(1)>\n\n"
    "## Footnotes (unsupported)\n\nText[^1].\n\n[^1]: note\n\n"
    "## Unclosed fence at the end of the file\n\n```rust\nfn never_closed() {\n"
)

numbered = "# A 1,200 item ordered list\n\n" + "".join(f"{i}. item {i}\n" for i in range(1, 1201))

tasks = """# Tasks to toggle

Press `x` or Enter on a task. The file on disk changes, so run `generate.py` again to reset.

- [ ] plain task
- [x] done task
- [X] uppercase X
- [ ] a very long task that wraps over several lines because it keeps going and going and
  going, and pressing x on any of its lines should toggle it
- [ ] parent
  - [ ] child
    - [x] grandchild
1. [ ] ordered task
2. [x] ordered done

> - [ ] task inside a quote

* [ ] star bullet
+ [ ] plus bullet

Not a task: [ ] in a sentence, and `- [ ]` in code.
"""

links = """# Links

- [anchor to the bottom](#bottom)
- [missing anchor](#does-not-exist)
- [wide.md](wide.md) and [math.md#display](math.md#display) (Backspace comes back)
- [missing file](nope.md)
- [a non-markdown file](generate.py)
- [relative up](../../README.md)
- [external](https://example.com) and <https://example.org> and <someone@example.com>
- [same URL twice](https://example.com) keeps one tag

Filler.

""" + "\n\n".join(f"Paragraph {i} to push the anchor off screen." for i in range(60)) + """

## Bottom

You arrived at the bottom.
"""

write("wide.md", wide)
write("mermaid.md", mermaid)
write("math.md", math)
write("nasty.md", nasty)
write("numbered.md", numbered)
write("tasks.md", tasks)
write("links.md", links)
write("crlf-bom.md", "﻿# CRLF and BOM\r\n\r\nLine one\r\nline two\r\n\r\n```\r\ncode\r\n```\r\n")
write("empty.md", "")
write("whitespace.md", "   \n\n\t\n   \n")
write("invalid-utf8.md", b"# Broken encoding\n\n\xff\xfe\xfa not UTF-8\n")
parts = [wide, mermaid, math, tasks]
write("big.md", "\n\n".join(parts) * 80)
