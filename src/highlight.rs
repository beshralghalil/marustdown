use std::hash::{DefaultHasher, Hash, Hasher};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(not(feature = "highlight"), allow(dead_code))]
pub enum Token {
    Keyword,
    String,
    Number,
    Comment,
    Type,
    Function,
    Constant,
    Operator,
    Tag,
    Attribute,
    Inserted,
    Deleted,
}

/// A styled range of one line: (start, end, token), in bytes.
pub type Span = (u32, u32, Token);

#[cfg(feature = "highlight")]
mod engine {
    use std::sync::OnceLock;

    use syntect::easy::ScopeRangeIterator;
    use syntect::parsing::{ParseState, Scope, ScopeStack, SyntaxReference, SyntaxSet};

    use super::{Span, Token};

    /// Scope prefixes, most specific first; a scope takes the first rule that matches it.
    const RULES: &[(&str, Token)] = &[
        ("comment", Token::Comment),
        ("constant.character.escape", Token::Constant),
        ("string", Token::String),
        ("markup.raw", Token::String),
        ("constant.numeric", Token::Number),
        ("constant", Token::Constant),
        ("support.constant", Token::Constant),
        ("keyword.operator", Token::Operator),
        ("keyword", Token::Keyword),
        ("storage", Token::Keyword),
        ("entity.name.function", Token::Function),
        ("support.function", Token::Function),
        ("variable.function", Token::Function),
        ("variable.annotation", Token::Function),
        ("entity.name.tag", Token::Tag),
        ("entity.other.attribute-name", Token::Attribute),
        ("entity.name", Token::Type),
        ("entity.other.inherited-class", Token::Type),
        ("support.type", Token::Type),
        ("support.class", Token::Type),
        ("markup.inserted", Token::Inserted),
        ("markup.deleted", Token::Deleted),
    ];

    /// Code-fence names that differ from the syntax's file extension or name.
    const ALIASES: &[(&str, &str)] = &[
        ("shell", "bash"),
        ("console", "bash"),
        ("shellsession", "bash"),
        ("zsh", "bash"),
        ("golang", "go"),
        ("c++", "cpp"),
        ("jsonc", "json"),
        ("docker", "dockerfile"),
    ];

    struct Syntaxes {
        set: SyntaxSet,
        rules: Vec<(Scope, Token)>,
    }

    /// Loaded on first use, so documents without code blocks never pay for it.
    fn syntaxes() -> &'static Syntaxes {
        static SYNTAXES: OnceLock<Syntaxes> = OnceLock::new();
        SYNTAXES.get_or_init(|| Syntaxes {
            set: two_face::syntax::extra_newlines(),
            rules: RULES
                .iter()
                .map(|&(s, t)| (Scope::new(s).expect("valid scope"), t))
                .collect(),
        })
    }

    fn find<'a>(set: &'a SyntaxSet, lang: &str) -> Option<&'a SyntaxReference> {
        let lang = ALIASES
            .iter()
            .find(|(a, _)| a.eq_ignore_ascii_case(lang))
            .map_or(lang, |(_, to)| to);
        set.find_syntax_by_token(lang)
            .filter(|s| s.name != "Plain Text")
    }

    impl Syntaxes {
        fn token(&self, stack: &ScopeStack) -> Option<Token> {
            stack.as_slice().iter().rev().find_map(|&scope| {
                self.rules
                    .iter()
                    .find(|(rule, _)| rule.is_prefix_of(scope))
                    .map(|&(_, t)| t)
            })
        }
    }

    /// Appends the spans of every line of `code`, closing each line in `ends`.
    /// Returns false when `lang` is not a known language.
    pub fn highlight(lang: &str, code: &str, spans: &mut Vec<Span>, ends: &mut Vec<u32>) -> bool {
        let syntaxes = syntaxes();
        let Some(syntax) = find(&syntaxes.set, lang) else {
            return false;
        };
        let mut state = ParseState::new(syntax);
        let mut stack = ScopeStack::new();
        let mut line = String::new();
        let mut parsing = true;
        for text in code.lines() {
            line.clear();
            line.push_str(text);
            line.push('\n');
            match state.parse_line(&line, &syntaxes.set) {
                Ok(ops) if parsing => {
                    for (range, op) in ScopeRangeIterator::new(&ops, &line) {
                        if stack.apply(op).is_err() {
                            parsing = false;
                            break;
                        }
                        let (start, end) = (range.start, range.end.min(text.len()));
                        if let Some(token) = syntaxes.token(&stack).filter(|_| start < end) {
                            push(spans, (start as u32, end as u32, token));
                        }
                    }
                }
                _ => parsing = false,
            }
            ends.push(spans.len() as u32);
        }
        true
    }

    /// Appends `span`, merging it into the previous one when they touch with the same token.
    fn push(spans: &mut Vec<Span>, span: Span) {
        match spans.last_mut() {
            Some(last) if last.1 == span.0 && last.2 == span.2 => last.1 = span.1,
            _ => spans.push(span),
        }
    }
}

#[cfg(not(feature = "highlight"))]
mod engine {
    pub fn highlight(_: &str, _: &str, _: &mut Vec<super::Span>, _: &mut Vec<u32>) -> bool {
        false
    }
}

/// Highlighted code blocks, kept across re-layouts. Highlighting doesn't depend on
/// the width, so a resize reuses every block and an edit redoes only changed ones.
#[derive(Default)]
pub struct Cache {
    blocks: Vec<Block>,
}

#[derive(Default)]
pub struct Block {
    key: Option<u64>,
    known: bool,
    spans: Vec<Span>,
    ends: Vec<u32>, // span count at the end of each line
}

impl Block {
    pub fn line(&self, i: usize) -> &[Span] {
        let start = i.checked_sub(1).map_or(0, |p| self.ends[p] as usize);
        let end = self.ends.get(i).map_or(start, |&e| e as usize);
        &self.spans[start..end]
    }
}

impl Cache {
    /// Spans for code block number `k`, or None if `lang` isn't a known language.
    pub fn block(&mut self, k: usize, lang: &str, code: &str) -> Option<&Block> {
        if !cfg!(feature = "highlight") || lang.is_empty() {
            return None;
        }
        if self.blocks.len() <= k {
            self.blocks.resize_with(k + 1, Block::default);
        }
        let mut hasher = DefaultHasher::new();
        (lang, code).hash(&mut hasher);
        let key = Some(hasher.finish());
        let block = &mut self.blocks[k];
        if block.key != key {
            block.spans.clear();
            block.ends.clear();
            block.known = engine::highlight(lang, code, &mut block.spans, &mut block.ends);
            block.key = key;
        }
        block.known.then_some(&*block)
    }

    /// Drops blocks past the first `n`, after a layout with `n` code blocks.
    pub fn truncate(&mut self, n: usize) {
        self.blocks.truncate(n);
    }
}

#[cfg(all(test, feature = "highlight"))]
mod tests {
    use super::*;

    fn tokens(lang: &str, code: &str) -> Vec<Vec<(String, Token)>> {
        let mut cache = Cache::default();
        let block = cache.block(0, lang, code).expect("known language");
        code.lines()
            .enumerate()
            .map(|(i, l)| {
                block
                    .line(i)
                    .iter()
                    .map(|&(s, e, t)| (l[s as usize..e as usize].to_owned(), t))
                    .collect()
            })
            .collect()
    }

    fn has(line: &[(String, Token)], text: &str, token: Token) -> bool {
        line.iter().any(|(s, t)| s == text && *t == token)
    }

    #[test]
    fn rust_tokens() {
        let got = tokens("rust", "fn main() { let x: Vec<u8> = \"hi\"; } // done");
        let line = &got[0];
        assert!(has(line, "fn", Token::Keyword));
        assert!(has(line, "main", Token::Function));
        assert!(has(line, "\"hi\"", Token::String));
        assert!(has(line, "// done", Token::Comment));
    }

    #[test]
    fn state_carries_across_lines() {
        let got = tokens("c", "int a; /* one\ntwo */ return 1;");
        assert!(has(&got[1], "two */", Token::Comment));
        assert!(has(&got[1], "1", Token::Number));
    }

    #[test]
    fn extra_languages_and_aliases() {
        assert!(
            tokens("toml", "a = \"b\"")[0]
                .iter()
                .any(|(_, t)| *t == Token::String)
        );
        assert!(!tokens("ts", "const x = 1")[0].is_empty());
        assert!(!tokens("shell", "echo hi # note")[0].is_empty());
        let diff = tokens("diff", "+added\n-removed");
        assert!(diff[0].iter().any(|(_, t)| *t == Token::Inserted));
        assert!(diff[1].iter().any(|(_, t)| *t == Token::Deleted));
    }

    #[test]
    fn unknown_and_plain() {
        let mut cache = Cache::default();
        assert!(cache.block(0, "no-such-language", "x").is_none());
        assert!(cache.block(1, "", "x").is_none());
        assert!(cache.block(2, "text", "x").is_none());
    }

    #[test]
    fn cache_reuses_unchanged_blocks() {
        let mut cache = Cache::default();
        let first = cache.block(0, "rust", "let a = 1;").unwrap().spans.as_ptr();
        assert_eq!(
            cache.block(0, "rust", "let a = 1;").unwrap().spans.as_ptr(),
            first
        );
        let changed = cache.block(0, "rust", "let a = \"s\";").unwrap();
        assert!(changed.line(0).iter().any(|s| s.2 == Token::String));
    }
}
