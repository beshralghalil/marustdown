use pulldown_cmark::Alignment;

/// Fits columns into `budget` by repeatedly shrinking the widest one.
/// Solved as a water level: every column is capped at the largest width that fits.
pub fn column_widths(natural: &[usize], budget: usize, out: &mut Vec<usize>) {
    out.clear();
    out.extend(natural.iter().map(|&w| w.max(1)));
    let capped = |cap: usize| out.iter().map(|&w| w.min(cap)).sum::<usize>();
    if out.iter().sum::<usize>() <= budget {
        return;
    }
    let (mut lo, mut hi) = (1, out.iter().copied().max().unwrap_or(1));
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        if capped(mid) <= budget {
            lo = mid
        } else {
            hi = mid - 1
        }
    }
    let mut spare = budget.saturating_sub(capped(lo));
    for w in out.iter_mut() {
        if *w > lo {
            *w = lo + usize::from(spare > 0);
            spare = spare.saturating_sub(1);
        }
    }
}

/// Left and right padding for a cell with `slack` free columns.
pub fn pad(align: Alignment, slack: usize) -> (usize, usize) {
    match align {
        Alignment::Right => (slack, 0),
        Alignment::Center => (slack / 2, slack - slack / 2),
        Alignment::Left | Alignment::None => (0, slack),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn widths(natural: &[usize], budget: usize) -> Vec<usize> {
        let mut out = Vec::new();
        column_widths(natural, budget, &mut out);
        out
    }

    #[test]
    fn fitting_table_keeps_natural_widths() {
        assert_eq!(widths(&[4, 7, 30], 50), [4, 7, 30]);
    }

    #[test]
    fn widest_column_shrinks_first() {
        assert_eq!(widths(&[4, 7, 30], 30), [4, 7, 19]);
        assert_eq!(widths(&[10, 10, 2], 16), [7, 7, 2]);
        assert_eq!(widths(&[10, 10, 2], 17), [8, 7, 2]);
    }

    #[test]
    fn never_below_one_column() {
        assert_eq!(widths(&[5, 5], 0), [1, 1]);
    }

    #[test]
    fn alignment_padding() {
        assert_eq!(pad(Alignment::Right, 3), (3, 0));
        assert_eq!(pad(Alignment::Center, 3), (1, 2));
        assert_eq!(pad(Alignment::None, 3), (0, 3));
    }
}
