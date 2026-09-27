use mermaid_text::RenderOptions;

use crate::safe;

/// Draws Mermaid source with box-drawing characters, compacted toward `width` columns
/// (it may still come out wider). `None` for unsupported or broken diagrams.
pub fn render(src: &str, width: usize, ascii: bool) -> Option<Vec<String>> {
    let options = RenderOptions {
        max_width: Some(width),
        ascii,
        ..RenderOptions::default()
    };
    let text = safe::catch(|| mermaid_text::render_with_options(src, &options))?.ok()?;
    let mut lines: Vec<String> = text.lines().map(|l| l.trim_end().to_owned()).collect();
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    while lines.first().is_some_and(String::is_empty) {
        lines.remove(0);
    }
    (!lines.is_empty()).then_some(lines)
}

/// Box lines and arrows, as opposed to the labels inside a diagram.
pub fn is_line(c: char) -> bool {
    matches!(c, '\u{2190}'..='\u{21ff}' | '\u{2500}'..='\u{257f}' | '\u{25b2}'..='\u{25c5}')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flowchart() {
        let lines = render("graph LR\n  A[Start] --> B[End]", 80, false).unwrap();
        let text = lines.join("\n");
        assert!(text.contains("Start") && text.contains("End") && text.contains('─'));
        assert!(lines.iter().all(|l| l == l.trim_end()));
    }

    #[test]
    fn ascii_mode() {
        let lines = render("graph LR\n  A --> B", 80, true).unwrap();
        assert!(lines.iter().all(|l| l.is_ascii()));
    }

    #[test]
    fn unsupported_or_broken_is_none() {
        assert!(render("notADiagram\n  x", 80, false).is_none());
        assert!(render("", 80, false).is_none());
    }

    #[test]
    fn renderer_panic_is_caught() {
        assert!(render("classDiagram\n  class Café\n  Café <|-- B", 80, false).is_none());
    }

    #[test]
    fn line_characters() {
        assert!(is_line('─') && is_line('▸') && is_line('→'));
        assert!(!is_line('A') && !is_line('█'));
    }
}
