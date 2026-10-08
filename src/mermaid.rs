use mermaid_text::RenderOptions;

use crate::blocks::{self, Context, Render};

/// Draws Mermaid diagrams with box-drawing characters, compacted toward the block width.
pub struct Mermaid;

impl Render for Mermaid {
    fn name(&self) -> &str {
        "mermaid"
    }

    fn render(&self, source: &str, cx: &Context) -> Option<Vec<String>> {
        let options = RenderOptions {
            max_width: Some(cx.width),
            ascii: cx.ascii,
            ..RenderOptions::default()
        };
        blocks::lines(&mermaid_text::render_with_options(source, &options).ok()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blocks::Cache;

    fn render(source: &str, ascii: bool) -> Option<Vec<String>> {
        let cx = Context {
            lang: "mermaid",
            width: 80,
            ascii,
        };
        Cache::default().render(&Mermaid, source, &cx)
    }

    #[test]
    fn flowchart() {
        let text = render("graph LR\n  A[Start] --> B[End]", false)
            .unwrap()
            .join("\n");
        assert!(text.contains("Start") && text.contains("End") && text.contains('─'));
    }

    #[test]
    fn ascii_mode() {
        assert!(
            render("graph LR\n  A --> B", true)
                .unwrap()
                .iter()
                .all(|l| l.is_ascii())
        );
    }

    #[test]
    fn unsupported_or_broken_is_none() {
        assert!(render("notADiagram\n  x", false).is_none());
        assert!(render("", false).is_none());
    }

    #[test]
    fn renderer_panic_is_caught() {
        assert!(render("classDiagram\n  class Café\n  Café <|-- B", false).is_none());
    }
}
