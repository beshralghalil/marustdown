use latex_to_unicode::ast::MathNode;
use latex_to_unicode::parser::Parser;
use latex_to_unicode::renderer::{RenderMode, Renderer};
use unicode_width::UnicodeWidthStr;

use crate::safe;

/// Inline math as one line of Unicode text, or the source if rendering fails.
pub fn inline(tex: &str) -> String {
    safe::catch(|| Renderer::new(RenderMode::Inline).render(&Parser::new(tex).parse()))
        .unwrap_or_else(|| tex.to_owned())
}

/// Display math as lines of Unicode text, or the source if rendering fails. Matrices
/// and cases are drawn in 2D, beside the rest of the formula and centered on its baseline.
pub fn display(tex: &str) -> Vec<String> {
    safe::catch(|| layout(tex)).unwrap_or_else(|| tex.lines().map(str::to_owned).collect())
}

fn layout(tex: &str) -> Vec<String> {
    let nodes = Parser::new(tex).parse();
    let (inline, block) = (
        Renderer::new(RenderMode::Inline),
        Renderer::new(RenderMode::Block),
    );
    let mut pieces: Vec<Vec<String>> = Vec::new();
    let mut start = 0;
    for (i, node) in nodes.iter().enumerate() {
        if matches!(node, MathNode::Matrix { .. }) {
            pieces.push(vec![inline.render(&nodes[start..i]).trim().to_owned()]);
            pieces.push(dedent(&block.render(&nodes[i..=i])));
            start = i + 1;
        }
    }
    pieces.push(vec![inline.render(&nodes[start..]).trim().to_owned()]);
    pieces.retain(|p| p.iter().any(|l| !l.is_empty()));
    beside(&pieces)
}

/// Lines with their common leading indentation and trailing spaces removed.
fn dedent(text: &str) -> Vec<String> {
    let indent = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start_matches(' ').len())
        .min()
        .unwrap_or(0);
    text.lines()
        .map(|l| l.get(indent..).unwrap_or("").trim_end().to_owned())
        .collect()
}

/// Joins blocks of lines left to right with one space between them, each vertically
/// centered on the tallest block's middle row.
fn beside(pieces: &[Vec<String>]) -> Vec<String> {
    let height = pieces.iter().map(Vec::len).max().unwrap_or(0);
    let mut rows = vec![String::new(); height];
    for (k, piece) in pieces.iter().enumerate() {
        let width = piece.iter().map(|l| l.width()).max().unwrap_or(0);
        let top = (height - 1) / 2 - (piece.len() - 1) / 2;
        for (r, row) in rows.iter_mut().enumerate() {
            if k > 0 {
                row.push(' ');
            }
            let line = r
                .checked_sub(top)
                .and_then(|i| piece.get(i))
                .map_or("", String::as_str);
            row.push_str(line);
            row.extend(std::iter::repeat_n(' ', width - line.width()));
        }
    }
    rows.iter().map(|r| r.trim_end().to_owned()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_unicode() {
        assert_eq!(inline(r"x^2 + y_i"), "x² + yᵢ");
        assert_eq!(inline(r"\alpha \leq \beta"), "α ≤ β");
    }

    #[test]
    fn display_single_line() {
        assert_eq!(display("x^2"), ["x²"]);
    }

    #[test]
    fn matrix_keeps_surrounding_text() {
        let m = r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}";
        assert_eq!(display(m), ["⎛ a   b ⎞", "⎝ c   d ⎠"]);
        assert_eq!(
            display(&format!("A = {m}")),
            ["A = ⎛ a   b ⎞", "    ⎝ c   d ⎠"]
        );
        let three = r"M = \begin{pmatrix} 1 \\ 2 \\ 3 \end{pmatrix} + v";
        assert_eq!(display(three), ["    ⎛ 1 ⎞", "M = ⎜ 2 ⎟ + v", "    ⎝ 3 ⎠"]);
    }

    #[test]
    fn delimiter_only_input_is_harmless() {
        for tex in ["$", "$$", " $$$ ", ""] {
            let _ = inline(tex);
            let _ = display(tex);
        }
    }
}
