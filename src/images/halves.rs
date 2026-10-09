//! The fallback for terminals without graphics: two pixels per cell, the top one as the
//! foreground of `▀` and the bottom one as its background.

use std::fmt::Write as _;

use image::{Rgba, RgbaImage, imageops};

pub fn encode(canvas: &RgbaImage, cols: u16, rows: u16) -> Vec<String> {
    let (w, h) = (u32::from(cols), 2 * u32::from(rows));
    let small = imageops::thumbnail(canvas, w, h);
    (0..u32::from(rows))
        .map(|row| {
            let mut out = String::new();
            let mut last = String::new();
            let mut sgr = String::new();
            for x in 0..w {
                let (top, bottom) = (small.get_pixel(x, 2 * row), small.get_pixel(x, 2 * row + 1));
                sgr.clear();
                sgr.push_str("\x1b[0");
                let glyph = match (visible(top), visible(bottom)) {
                    (false, false) => " ",
                    (true, false) => {
                        color(&mut sgr, top, 38);
                        "▀"
                    }
                    (false, true) => {
                        color(&mut sgr, bottom, 38);
                        "▄"
                    }
                    (true, true) => {
                        color(&mut sgr, top, 38);
                        color(&mut sgr, bottom, 48);
                        "▀"
                    }
                };
                sgr.push('m');
                if sgr != last {
                    out.push_str(&sgr);
                    std::mem::swap(&mut sgr, &mut last);
                }
                out.push_str(glyph);
            }
            out.push_str("\x1b[0m");
            out
        })
        .collect()
}

fn visible(p: &Rgba<u8>) -> bool {
    p.0[3] >= 128
}

/// `;38;…` or `;48;…` setting `p` as a 24-bit color.
fn color(out: &mut String, p: &Rgba<u8>, base: u8) {
    let [r, g, b, _] = p.0;
    let _ = write!(out, ";{base};2;{r};{g};{b}");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canvas(top: [u8; 4], bottom: [u8; 4]) -> RgbaImage {
        RgbaImage::from_fn(2, 2, |_, y| Rgba(if y == 0 { top } else { bottom }))
    }

    #[test]
    fn two_pixels_per_cell() {
        let rows = encode(&canvas([255, 0, 0, 255], [0, 0, 255, 255]), 2, 1);
        assert_eq!(rows, ["\x1b[0;38;2;255;0;0;48;2;0;0;255m▀▀\x1b[0m"]);
    }

    #[test]
    fn transparent_pixels_show_the_background() {
        let rows = encode(&canvas([0, 0, 0, 0], [9, 9, 9, 255]), 1, 1);
        assert_eq!(rows, ["\x1b[0;38;2;9;9;9m▄\x1b[0m"]);
        let rows = encode(&canvas([0, 0, 0, 0], [0, 0, 0, 0]), 1, 1);
        assert_eq!(rows, ["\x1b[0m \x1b[0m"]);
    }
}
