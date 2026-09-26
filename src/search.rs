/// Substring matcher with smart case: case-insensitive (ASCII) unless the query has uppercase.
pub struct Matcher {
    needle: String,
    fold: bool,
}

impl Matcher {
    pub fn new(query: &str) -> Self {
        Matcher {
            needle: query.to_owned(),
            fold: !query.chars().any(char::is_uppercase),
        }
    }

    /// Next match at or after byte `from`, as a byte range.
    pub fn find(&self, hay: &str, from: usize) -> Option<(usize, usize)> {
        let n = self.needle.len();
        if n == 0 || from > hay.len() {
            return None;
        }
        let start = if self.fold {
            let (h, needle) = (hay.as_bytes(), self.needle.as_bytes());
            (from..=h.len().checked_sub(n)?).find(|&i| h[i..i + n].eq_ignore_ascii_case(needle))?
        } else {
            from + hay[from..].find(&self.needle)?
        };
        Some((start, start + n))
    }

    pub fn matches<'h>(&'h self, hay: &'h str) -> impl Iterator<Item = (usize, usize)> + 'h {
        let mut from = 0;
        std::iter::from_fn(move || {
            let hit = self.find(hay, from)?;
            from = hit.1;
            Some(hit)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smart_case() {
        assert_eq!(Matcher::new("rust").find("Use Rust", 0), Some((4, 8)));
        assert_eq!(Matcher::new("Rust").find("use rust", 0), None);
    }

    #[test]
    fn all_matches() {
        let m = Matcher::new("ab");
        assert_eq!(
            m.matches("ab xab ab").collect::<Vec<_>>(),
            [(0, 2), (4, 6), (7, 9)]
        );
        assert_eq!(Matcher::new("").matches("abc").count(), 0);
    }

    #[test]
    fn non_ascii_haystack() {
        assert_eq!(Matcher::new("é").find("café", 0), Some((3, 5)));
    }
}
