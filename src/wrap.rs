use unicode_width::UnicodeWidthChar;

/// Clears `out`, then appends the byte offset where each line after the first starts.
/// Greedy fill at spaces; `\n` forces a break; words wider than `width` are hard-split.
/// The space at a soft break ends the previous line, so callers trim it.
pub fn breaks(text: &str, width: usize, out: &mut Vec<u32>) {
    out.clear();
    let width = width.max(1);
    let mut cols = 0;
    let mut space: Option<(usize, usize)> = None; // (line start after it, columns up to it)
    for (i, c) in text.char_indices() {
        match c {
            '\n' => {
                if i + 1 < text.len() {
                    out.push((i + 1) as u32);
                }
                cols = 0;
                space = None;
            }
            ' ' if cols + 1 > width => {
                if i + 1 < text.len() {
                    out.push((i + 1) as u32);
                }
                cols = 0;
                space = None;
            }
            ' ' => {
                cols += 1;
                space = Some((i + 1, cols));
            }
            _ => {
                let w = c.width().unwrap_or(0);
                if cols + w > width && cols > 0 {
                    match space.take() {
                        Some((at, used)) => {
                            out.push(at as u32);
                            cols -= used;
                            if cols + w > width {
                                out.push(i as u32);
                                cols = 0;
                            }
                        }
                        None => {
                            out.push(i as u32);
                            cols = 0;
                        }
                    }
                }
                cols += w;
            }
        }
    }
}

/// Longest prefix of `s` that fits in `cols` columns: (byte length, columns used).
pub fn fit(s: &str, cols: usize) -> (usize, usize) {
    let mut used = 0;
    for (i, c) in s.char_indices() {
        let w = c.width().unwrap_or(0);
        if used + w > cols {
            return (i, used);
        }
        used += w;
    }
    (s.len(), used)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(text: &str, width: usize) -> Vec<u32> {
        let mut out = vec![99];
        breaks(text, width, &mut out);
        out
    }

    #[test]
    fn greedy_fill() {
        assert_eq!(run("aaa bbb ccc", 7), [8]);
        assert_eq!(run("aaa bbb ccc", 100), [] as [u32; 0]);
        assert_eq!(run("", 10), [] as [u32; 0]);
        assert_eq!(run("aa b", 3), [3]);
    }

    #[test]
    fn hard_split() {
        assert_eq!(run("abcdefghij", 4), [4, 8]);
        assert_eq!(run("ab cdefgh", 3), [3, 6]);
    }

    #[test]
    fn measures_columns_not_bytes() {
        assert_eq!(run("日本語 abc", 6), [10]);
        assert_eq!(run("ééééé", 2), [4, 8]);
    }

    #[test]
    fn newline_forces_break() {
        assert_eq!(run("a\nb", 10), [2]);
        assert_eq!(run("a\n", 10), [] as [u32; 0]);
    }

    #[test]
    fn fit_prefix() {
        assert_eq!(fit("héllo", 3), (4, 3));
        assert_eq!(fit("日本", 3), (3, 2));
        assert_eq!(fit("ab", 5), (2, 2));
    }
}
