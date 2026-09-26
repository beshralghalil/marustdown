#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Token {
    Keyword,
    String,
    Number,
    Comment,
    Type,
}

pub type Span = (u32, u32, Token);

struct Lang {
    keywords: &'static [&'static str],
    comments: &'static [&'static str],
    block: Option<(&'static str, &'static str)>,
    quotes: &'static [u8],
    types: bool,     // capitalized identifiers are types
    fold: bool,      // keywords are case-insensitive
    lifetimes: bool, // `'a` is a lifetime unless it closes like a char literal
}

const C_BLOCK: Option<(&str, &str)> = Some(("/*", "*/"));

const RUST: Lang = Lang {
    keywords: &[
        "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
        "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move",
        "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait",
        "true", "type", "unsafe", "use", "where", "while",
    ],
    comments: &["//"],
    block: C_BLOCK,
    quotes: b"\"'",
    types: true,
    fold: false,
    lifetimes: true,
};

const PYTHON: Lang = Lang {
    keywords: &[
        "False", "None", "True", "and", "as", "assert", "async", "await", "break", "case", "class",
        "continue", "def", "del", "elif", "else", "except", "finally", "for", "from", "global",
        "if", "import", "in", "is", "lambda", "match", "nonlocal", "not", "or", "pass", "raise",
        "return", "self", "try", "while", "with", "yield",
    ],
    comments: &["#"],
    block: None,
    quotes: b"\"'",
    types: true,
    fold: false,
    lifetimes: false,
};

const JS: Lang = Lang {
    keywords: &[
        "as",
        "async",
        "await",
        "break",
        "case",
        "catch",
        "class",
        "const",
        "continue",
        "default",
        "delete",
        "do",
        "else",
        "enum",
        "export",
        "extends",
        "false",
        "finally",
        "for",
        "from",
        "function",
        "if",
        "implements",
        "import",
        "in",
        "instanceof",
        "interface",
        "let",
        "new",
        "null",
        "of",
        "private",
        "protected",
        "public",
        "readonly",
        "return",
        "static",
        "super",
        "switch",
        "this",
        "throw",
        "true",
        "try",
        "type",
        "typeof",
        "undefined",
        "var",
        "void",
        "while",
        "yield",
    ],
    comments: &["//"],
    block: C_BLOCK,
    quotes: b"\"'`",
    types: true,
    fold: false,
    lifetimes: false,
};

const GO: Lang = Lang {
    keywords: &[
        "break",
        "case",
        "chan",
        "const",
        "continue",
        "default",
        "defer",
        "else",
        "fallthrough",
        "false",
        "for",
        "func",
        "go",
        "goto",
        "if",
        "import",
        "interface",
        "map",
        "nil",
        "package",
        "range",
        "return",
        "select",
        "struct",
        "switch",
        "true",
        "type",
        "var",
    ],
    comments: &["//"],
    block: C_BLOCK,
    quotes: b"\"'`",
    types: true,
    fold: false,
    lifetimes: false,
};

const C: Lang = Lang {
    keywords: &[
        "auto",
        "bool",
        "break",
        "case",
        "char",
        "class",
        "const",
        "continue",
        "default",
        "define",
        "delete",
        "do",
        "double",
        "else",
        "enum",
        "extern",
        "false",
        "float",
        "for",
        "goto",
        "if",
        "include",
        "inline",
        "int",
        "long",
        "namespace",
        "new",
        "nullptr",
        "private",
        "protected",
        "public",
        "return",
        "short",
        "signed",
        "sizeof",
        "static",
        "struct",
        "switch",
        "template",
        "this",
        "true",
        "typedef",
        "typename",
        "union",
        "unsigned",
        "virtual",
        "void",
        "volatile",
        "while",
    ],
    comments: &["//"],
    block: C_BLOCK,
    quotes: b"\"'",
    types: true,
    fold: false,
    lifetimes: false,
};

const SHELL: Lang = Lang {
    keywords: &[
        "alias", "break", "case", "continue", "declare", "do", "done", "elif", "else", "esac",
        "exit", "export", "fi", "for", "function", "if", "in", "local", "readonly", "return",
        "select", "set", "shift", "source", "then", "unset", "until", "while",
    ],
    comments: &["#"],
    block: None,
    quotes: b"\"'`",
    types: false,
    fold: false,
    lifetimes: false,
};

const JSON: Lang = Lang {
    keywords: &["true", "false", "null"],
    comments: &["//"],
    block: C_BLOCK,
    quotes: b"\"",
    types: false,
    fold: false,
    lifetimes: false,
};

const CONF: Lang = Lang {
    keywords: &["true", "false", "null", "yes", "no", "on", "off"],
    comments: &["#"],
    block: None,
    quotes: b"\"'",
    types: false,
    fold: false,
    lifetimes: false,
};

const SQL: Lang = Lang {
    keywords: &[
        "alter", "and", "as", "asc", "by", "create", "delete", "desc", "distinct", "drop", "from",
        "group", "having", "in", "index", "inner", "insert", "into", "is", "join", "key", "left",
        "limit", "not", "null", "on", "or", "order", "primary", "right", "select", "set", "table",
        "union", "update", "values", "where",
    ],
    comments: &["--"],
    block: C_BLOCK,
    quotes: b"'\"",
    types: false,
    fold: true,
    lifetimes: false,
};

const LUA: Lang = Lang {
    keywords: &[
        "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto", "if",
        "in", "local", "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
    ],
    comments: &["--"],
    block: None,
    quotes: b"\"'",
    types: false,
    fold: false,
    lifetimes: false,
};

fn lang(name: &str) -> Option<&'static Lang> {
    let lang = match name.to_ascii_lowercase().as_str() {
        "rust" | "rs" => &RUST,
        "python" | "py" => &PYTHON,
        "javascript" | "js" | "jsx" | "mjs" | "typescript" | "ts" | "tsx" => &JS,
        "go" | "golang" => &GO,
        "c" | "h" | "cpp" | "c++" | "cc" | "hpp" => &C,
        "bash" | "sh" | "shell" | "zsh" | "console" => &SHELL,
        "json" | "jsonc" => &JSON,
        "toml" | "yaml" | "yml" | "ini" => &CONF,
        "sql" => &SQL,
        "lua" => &LUA,
        _ => return None,
    };
    Some(lang)
}

/// Line-by-line token scanner; carries block-comment state between lines.
pub struct Highlighter {
    lang: &'static Lang,
    in_block: bool,
}

impl Highlighter {
    pub fn new(lang_name: &str) -> Option<Self> {
        lang(lang_name).map(|lang| Highlighter {
            lang,
            in_block: false,
        })
    }

    /// Replaces `out` with the non-plain spans of `line`, in order.
    pub fn line(&mut self, line: &str, out: &mut Vec<Span>) {
        out.clear();
        let b = line.as_bytes();
        let n = b.len();
        let lang = self.lang;
        let mut push = |s: usize, e: usize, t: Token| out.push((s as u32, e as u32, t));
        let mut i = 0;

        if self.in_block {
            let close = lang.block.map_or("", |(_, c)| c);
            match find(b, close.as_bytes()) {
                Some(e) => {
                    i = e + close.len();
                    self.in_block = false;
                    push(0, i, Token::Comment);
                }
                None => return push(0, n, Token::Comment),
            }
        }

        while i < n {
            let c = b[i];
            let rest = &b[i..];
            let word_start = i == 0 || !is_ident(b[i - 1]);

            let comment = lang.comments.iter().any(|m| {
                rest.starts_with(m.as_bytes())
                    && (m.as_bytes()[0] != b'#' || i == 0 || b[i - 1].is_ascii_whitespace())
            });
            if comment {
                return push(i, n, Token::Comment);
            }
            if let Some((open, close)) = lang.block
                && rest.starts_with(open.as_bytes())
            {
                let from = i + open.len();
                match find(&b[from..], close.as_bytes()) {
                    Some(e) => {
                        let end = from + e + close.len();
                        push(i, end, Token::Comment);
                        i = end;
                        continue;
                    }
                    None => {
                        self.in_block = true;
                        return push(i, n, Token::Comment);
                    }
                }
            }
            if lang.quotes.contains(&c) && !(lang.lifetimes && c == b'\'' && !is_char_literal(rest))
            {
                let end = string_end(b, i);
                push(i, end, Token::String);
                i = end;
                continue;
            }
            if c.is_ascii_digit() && word_start {
                let end = scan(b, i, |c| is_ident(c) || c == b'.');
                push(i, end, Token::Number);
                i = end;
                continue;
            }
            if (c.is_ascii_alphabetic() || c == b'_') && word_start {
                let end = scan(b, i, is_ident);
                let word = &line[i..end];
                let keyword = lang.keywords.iter().any(|k| {
                    if lang.fold {
                        k.eq_ignore_ascii_case(word)
                    } else {
                        *k == word
                    }
                });
                if keyword {
                    push(i, end, Token::Keyword);
                } else if lang.types && c.is_ascii_uppercase() {
                    push(i, end, Token::Type);
                }
                i = end;
                continue;
            }
            i += 1;
        }
    }
}

fn is_ident(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

fn scan(b: &[u8], from: usize, keep: impl Fn(u8) -> bool) -> usize {
    b[from..]
        .iter()
        .position(|&c| !keep(c))
        .map_or(b.len(), |p| from + p)
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// End of the string literal opened at `start` (exclusive), or the line end.
fn string_end(b: &[u8], start: usize) -> usize {
    let quote = b[start];
    let mut i = start + 1;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 2,
            c if c == quote => return i + 1,
            _ => i += 1,
        }
    }
    b.len()
}

/// `'x'` or `'\n'` rather than a lifetime like `'a`.
fn is_char_literal(rest: &[u8]) -> bool {
    let Some(&first) = rest.get(1) else {
        return false;
    };
    let len = match first {
        b'\\' => return true,
        0..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        _ => 4,
    };
    rest.get(1 + len) == Some(&b'\'')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans(lang: &str, lines: &[&str]) -> Vec<Vec<(String, Token)>> {
        let mut h = Highlighter::new(lang).unwrap();
        let mut out = Vec::new();
        lines
            .iter()
            .map(|l| {
                h.line(l, &mut out);
                out.iter()
                    .map(|&(s, e, t)| (l[s as usize..e as usize].to_owned(), t))
                    .collect()
            })
            .collect()
    }

    fn tok(s: &str, t: Token) -> (String, Token) {
        (s.to_owned(), t)
    }

    #[test]
    fn rust_line() {
        let got = spans("rust", &["let x: Vec<u8> = \"hi\"; // done"]);
        assert_eq!(
            got[0],
            [
                tok("let", Token::Keyword),
                tok("Vec", Token::Type),
                tok("\"hi\"", Token::String),
                tok("// done", Token::Comment),
            ]
        );
    }

    #[test]
    fn lifetimes_and_chars() {
        let got = spans("rust", &["fn f<'a>(c: char) { 'x' }"]);
        assert_eq!(
            got[0],
            [tok("fn", Token::Keyword), tok("'x'", Token::String)]
        );
    }

    #[test]
    fn block_comment_spans_lines() {
        let got = spans("c", &["int a; /* one", "two */ return 1;"]);
        assert_eq!(
            got[0],
            [tok("int", Token::Keyword), tok("/* one", Token::Comment)]
        );
        assert_eq!(
            got[1],
            [
                tok("two */", Token::Comment),
                tok("return", Token::Keyword),
                tok("1", Token::Number),
            ]
        );
    }

    #[test]
    fn hash_comment_needs_word_boundary() {
        let got = spans("bash", &["echo $# # note"]);
        assert_eq!(got[0], [tok("# note", Token::Comment)]);
    }

    #[test]
    fn unknown_language() {
        assert!(Highlighter::new("brainfuck").is_none());
    }

    #[test]
    fn non_ascii_is_safe() {
        let got = spans("python", &["s = 'é' # ünï"]);
        assert_eq!(
            got[0],
            [tok("'é'", Token::String), tok("# ünï", Token::Comment)]
        );
    }
}
