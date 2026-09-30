use std::time::{Duration, Instant};

/// Time between animation frames (~60 fps).
pub const FRAME: Duration = Duration::from_millis(16);

/// A vertical scroll gliding from one top line to another with an ease-out curve.
pub struct Glide {
    from: usize,
    to: usize,
    start: Instant,
    duration: Duration,
}

impl Glide {
    pub fn new(from: usize, to: usize, duration: Duration) -> Self {
        Glide {
            from,
            to,
            start: Instant::now(),
            duration,
        }
    }

    /// Top line to show at `now`, or `None` once the glide has arrived.
    pub fn at(&self, now: Instant) -> Option<usize> {
        let t = now.duration_since(self.start).as_secs_f64() / self.duration.as_secs_f64();
        if t >= 1.0 {
            return None;
        }
        let eased = 1.0 - (1.0 - t).powi(3);
        let (from, to) = (self.from as f64, self.to as f64);
        Some((from + (to - from) * eased).round() as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glide(from: usize, to: usize) -> (Glide, Instant) {
        let g = Glide::new(from, to, Duration::from_millis(100));
        let start = g.start;
        (g, start)
    }

    #[test]
    fn starts_at_from_and_arrives() {
        let (g, start) = glide(10, 50);
        assert_eq!(g.at(start), Some(10));
        assert_eq!(g.at(start + Duration::from_millis(100)), None);
    }

    #[test]
    fn eases_out_monotonically() {
        let (g, start) = glide(0, 100);
        let steps: Vec<usize> = (0..10)
            .filter_map(|ms| g.at(start + Duration::from_millis(ms * 10)))
            .collect();
        assert!(steps.windows(2).all(|w| w[0] <= w[1]));
        assert!(g.at(start + Duration::from_millis(50)).unwrap() > 50);
    }

    #[test]
    fn glides_upward() {
        let (g, start) = glide(80, 20);
        let mid = g.at(start + Duration::from_millis(30)).unwrap();
        assert!((20..80).contains(&mid));
    }
}
