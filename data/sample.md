# marustdown feature tour

This document uses every markdown construct marustdown renders. Open it with
`mar data/sample.md` and try the keys as you read: `j`/`k` move the cursor, `]]` and
`[[` jump between headings, `o` opens the outline, `/` searches, `x` checks off a task,
`y` copies a code block, and `e` opens this file in your editor.

---

## Headings

The six ATX levels follow. Every heading appears in the outline (`o`) and in the
breadcrumb on the status bar.

# Heading level 1
## Heading level 2
### Heading level 3
#### Heading level 4
##### Heading level 5
###### Heading level 6

Setext heading, level 1
=======================

Setext heading, level 2
-----------------------

### A heading with `inline code`, **bold** and a [link](#links)

## Paragraphs and line breaks

A paragraph is one or more lines of text. Single line breaks inside a paragraph are
soft, so these three source lines
flow together
into one wrapped paragraph that fills the content width and reflows when you resize the terminal.

A line ending in two spaces  
forces a hard break, and so does a trailing backslash\
like this. An inline `<br>` works too:<br>this sentence starts on a new line.

Very long words are split rather than overflowing the page:
Pneumonoultramicroscopicsilicovolcanoconiosis_and_a_few_more_characters_to_make_it_really_long.

## Inline formatting

- **Bold** and __bold__
- *Italic* and _italic_
- ***Bold italic*** and **bold with *nested italic* inside**
- ~~Strikethrough~~ and ~~**bold strikethrough**~~
- `Inline code`, and code with backticks inside: `` `tick` ``
- Escaped characters: \*not italic\*, \`not code\`, \# not a heading
- Unicode is measured by display width: 日本語のテキスト, café, naïve, emoji 🚀✨

## Links

- Inline link: [pulldown-cmark](https://github.com/pulldown-cmark/pulldown-cmark)
- Link with a title: [Rust](https://www.rust-lang.org "The Rust language")
- Reference link: [CommonMark spec][spec] and a collapsed reference: [GFM][]
- Autolink: <https://github.github.com/gfm/>
- Email autolink: <someone@example.com>
- Link to a section of this document: [jump to tables](#tables)
- Relative link to another file: [the plan](../plan.md)
- Link with **bold text** inside: [**bold link**](https://example.com)

[spec]: https://spec.commonmark.org/
[GFM]: https://github.github.com/gfm/

In terminals that support OSC 8 (kitty, WezTerm, iTerm2, GNOME Terminal and others),
links are clickable.

## Images

Terminals can't show the image itself, so marustdown shows its alt text as a link to
the file:

![Ferris the crab](https://rustacean.net/assets/rustacean-flat-happy.png)

An image inside a link: [![build badge](https://img.shields.io/badge/build-passing-green)](https://example.com/ci)

## Blockquotes

> A plain blockquote is drawn with a bar and italic text.
> It can span several lines, and wraps like any paragraph when the text is long enough
> to reach the edge of the content area.
>
> A second paragraph inside the same quote.

> Quotes can hold other blocks:
>
> - a list item
> - another one
>
> ```sh
> echo "code inside a quote"
> ```
>
> > And quotes nest inside quotes.
> > > As deep as you like.

## Alerts

> [!NOTE]
> Useful information that users should know, even when skimming content.

> [!TIP]
> Helpful advice for doing things better or more easily.

> [!IMPORTANT]
> Key information users need to know to achieve their goal.

> [!WARNING]
> Urgent info that needs immediate user attention to avoid problems.

> [!CAUTION]
> Advises about risks or negative outcomes of certain actions.
>
> Alerts can hold several paragraphs, `code`, and **formatting**.

## Lists

### Unordered, nested

- First level
- Bullets change with depth
  - Second level
  - Another item
    - Third level
      - Fourth level wraps back to the first bullet shape
- Back at the top. Long items wrap with a hanging indent, so the continuation lines
  line up under the text rather than under the bullet.

### Ordered

1. Numbers are right-aligned
2. Second
3. Third
4. Fourth
5. Fifth
6. Sixth
7. Seventh
8. Eighth
9. Ninth
10. Tenth: the single digits above are padded to line up with this one
11. Eleventh

A list can start at any number:

7. Seven
8. Eight
9. Nine

### Loose lists

- A loose list has blank lines between items.

- Each item can hold several paragraphs.

  This is a second paragraph in the same item.

- And other blocks too:

  ```python
  print("code inside a list item")
  ```

### Mixed nesting

1. Install the tool
   - with cargo: `cargo install --path .`
   - or build it: `cargo build --release`
2. Configure it
   > Every setting is optional, so you can start with an empty file.
3. Read markdown
   1. in the pager
   2. or with `--cat`

## Task lists

Move the cursor onto a task and press `x` (or `Enter`) to check it off. The change is
written back to this file.

- [x] Parse markdown
- [x] Lay out text
- [ ] Toggle me with `x`
- [ ] A long task wraps across several lines, and the whole item stays toggleable, so
  pressing `x` on any of its lines works
- [ ] Tasks nest
  - [x] Subtask done
  - [ ] Subtask open

1. [ ] Ordered task lists work as well
2. [x] Like this one

## Code

### Fenced blocks with syntax highlighting

```rust
use std::collections::HashMap;

/// Counts words in a string.
fn word_count(text: &str) -> HashMap<&str, usize> {
    let mut counts = HashMap::new();
    for word in text.split_whitespace() {
        *counts.entry(word).or_insert(0) += 1; // count it
    }
    counts
}

fn longest<'a>(a: &'a str, b: &'a str) -> &'a str {
    if a.len() > b.len() { a } else { b }
}
```

```python
from dataclasses import dataclass

@dataclass
class Point:
    x: float = 0.0
    y: float = 0.0

    def distance(self, other: "Point") -> float:
        # Euclidean distance
        return ((self.x - other.x) ** 2 + (self.y - other.y) ** 2) ** 0.5
```

```javascript
const fetchUser = async (id) => {
  const res = await fetch(`/api/users/${id}`);
  if (!res.ok) throw new Error("request failed"); // bail out
  return res.json();
};
```

```typescript
interface User {
  id: number;
  name: string;
}

export function greet(user: User): string {
  return `Hello, ${user.name}`;
}
```

```go
package main

import "fmt"

func main() {
	ch := make(chan int, 3)
	for i := 0; i < 3; i++ {
		ch <- i * 10
	}
	close(ch)
	for v := range ch {
		fmt.Println(v) // tabs are expanded to the configured tab width
	}
}
```

```c
#include <stdio.h>

/* A classic block comment
   spanning several lines. */
int main(void) {
    const char *msg = "hello";
    printf("%s, %d\n", msg, 42);
    return 0;
}
```

```bash
#!/usr/bin/env bash
set -euo pipefail

for file in *.md; do
  echo "rendering $file" # comment
  mar --cat "$file" > "${file%.md}.ansi"
done
```

```json
{
  "name": "marustdown",
  "version": "0.1.0",
  "features": ["pager", "search", "tasks"],
  "stable": true,
  "license": null
}
```

```toml
# ~/.config/marustdown/config.toml
theme = "dark"

[layout]
width = 100
center = true
```

```yaml
server:
  port: 8080
  debug: false # overridden in production
```

```sql
SELECT name, COUNT(*) AS total
FROM orders
WHERE status = 'shipped'
GROUP BY name
ORDER BY total DESC
LIMIT 10;
```

```lua
local function greet(name)
  -- say hello
  return "Hello, " .. name
end
```

### Without a language

```
No language means no highlighting.
Line numbers and the box are still drawn.
```

### Indented code block

    Four spaces of indentation make a code block too.
    It has no language label.

### Long lines are cut off

```text
This line is far too long to fit in the box, so it is cut off at the edge with an ellipsis instead of wrapping onto a second line.
```

## Tables

| Left aligned | Centered | Right aligned |
|:-------------|:--------:|--------------:|
| apples       | 3        | $1.20         |
| bananas      | 12       | $0.50         |
| cherries     | 250      | $15.00        |

Cells can hold inline formatting:

| Flag        | Default | Description                        |
|-------------|---------|------------------------------------|
| `--cat`     | off     | Print styled output and **exit**   |
| `--width`   | `90`    | Maximum content width, *in columns* |
| `--theme`   | `dark`  | One of [dark, light, ansi](#themes) |
| `--no-icons`| off     | ~~Nerd Font~~ ASCII glyphs          |

Wide tables shrink their widest columns to fit, and wrap the text inside:

| Feature | Status | Notes |
|---------|:------:|-------|
| Wrapping | ✔ | Cell text wraps inside its column when the table is wider than the screen, so nothing gets cut off or spills past the right edge. |
| Line breaks | ✔ | A `<br>` in a cell<br>starts a new line within that cell. |
| Alignment | ✔ | Left, center and right alignment from the delimiter row are respected on every wrapped line. |

## Horizontal rules

Three or more dashes, asterisks or underscores:

---

***

___

## HTML

Inline tags such as <kbd>Ctrl</kbd>+<kbd>C</kbd> are dropped and their text kept. HTML
blocks aren't rendered:

<details>
<summary>This whole block is hidden in the viewer</summary>
Nothing between the tags appears.
</details>

## Themes

Run this file with `mar --theme light data/sample.md` or `mar --theme ansi data/sample.md`
to compare the built-in presets, and with `--no-icons` or `--no-color` for the plain
fallbacks. See the README for the full configuration reference.

## Not supported

Footnotes[^1] and math such as $e^{i\pi} + 1 = 0$ are shown as plain text.

[^1]: This footnote definition is shown as a plain paragraph.
