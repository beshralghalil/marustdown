# marustdown

A fast terminal markdown viewer with **syntax highlighting**, task lists you can check
off, and an outline for jumping around. See the [README](../README.md) for more.

## Code

```rust
use std::collections::HashMap;

/// Counts how often each word appears.
fn word_count(text: &str) -> HashMap<&str, usize> {
    let mut counts = HashMap::new();
    for word in text.split_whitespace() {
        *counts.entry(word).or_insert(0) += 1; // one more
    }
    counts
}
```

> [!TIP]
> Press `y` to copy a code block to the clipboard, even over SSH.

## Math

LaTeX becomes Unicode: Euler's identity $e^{i\pi} + 1 = 0$, a sum
$\sum_{i=1}^{n} i = \frac{n(n+1)}{2}$, or sets like $x \in \mathbb{R}$.

$$
\int_0^\infty e^{-x^2}\,dx = \frac{\sqrt{\pi}}{2}
$$

$$
A = \begin{pmatrix} a & b \\ c & d \end{pmatrix}
$$

## Tasks

- [x] Render markdown in the terminal
- [ ] Check off tasks with `x`
- [ ] Jump anywhere with the outline

## Keys

| Key | Action |
|:---:|--------|
| `x` | Toggle the task under the cursor |
| `o` | Open the outline |
| `f` | Follow a link from the keyboard |

## Configuration

### Themes

Pick `dark`, `light` or `ansi`, or write your own theme file.

### Key bindings

Every key can be rebound in `~/.config/marustdown/config.toml`.
