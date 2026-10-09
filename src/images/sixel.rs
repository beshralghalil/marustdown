//! Sixel graphics: one image per row of cells, so a partly visible image shows just the
//! rows in view. All rows share one NeuQuant palette.

use std::fmt::Write as _;

use color_quant::NeuQuant;
use image::RgbaImage;

const COLORS: usize = 255;
const CLEAR: u16 = u16::MAX;

pub fn encode(canvas: &RgbaImage, cell_h: u32) -> Vec<String> {
    let rows = (canvas.height() / cell_h) as usize;
    let samples: Vec<u8> = canvas
        .pixels()
        .filter(|p| opaque(p.0))
        .flat_map(|p| p.0)
        .collect();
    if samples.is_empty() {
        return vec![String::new(); rows];
    }
    let quant = NeuQuant::new(10, COLORS, &samples);
    let palette = quant.color_map_rgb();
    let pixels: Vec<u16> = canvas
        .pixels()
        .map(|p| {
            if opaque(p.0) {
                quant.index_of(&p.0) as u16
            } else {
                CLEAR
            }
        })
        .collect();
    let width = canvas.width() as usize;
    pixels
        .chunks(cell_h as usize * width)
        .take(rows)
        .map(|strip| image(strip, width, &palette))
        .collect()
}

fn opaque(p: [u8; 4]) -> bool {
    p[3] >= 128
}

/// One sixel image of palette indices, `width` wide; `CLEAR` pixels stay transparent.
fn image(pixels: &[u16], width: usize, palette: &[u8]) -> String {
    let mut out = format!("\x1bP0;1;0q\"1;1;{width};{}", pixels.len() / width);
    let mut used = vec![false; palette.len() / 3];
    for &p in pixels.iter().filter(|&&p| p != CLEAR) {
        used[p as usize] = true;
    }
    let percent = |v: u8| u32::from(v) * 100 / 255;
    for (c, rgb) in palette.chunks(3).enumerate().filter(|&(c, _)| used[c]) {
        let (r, g, b) = (percent(rgb[0]), percent(rgb[1]), percent(rgb[2]));
        let _ = write!(out, "#{c};2;{r};{g};{b}");
    }
    let bands = pixels.chunks(6 * width);
    let last = bands.len().saturating_sub(1);
    for (k, band) in bands.enumerate() {
        used.fill(false);
        for &p in band.iter().filter(|&&p| p != CLEAR) {
            used[p as usize] = true;
        }
        for c in (0..used.len()).filter(|&c| used[c]) {
            let _ = write!(out, "#{c}");
            let mut run = (b'?', 0);
            for x in 0..width {
                let bits = (0..band.len() / width)
                    .filter(|&y| band[y * width + x] == c as u16)
                    .fold(0, |bits, y| bits | 1 << y);
                let sixel = b'?' + bits;
                if sixel != run.0 {
                    repeat(&mut out, run);
                    run = (sixel, 0);
                }
                run.1 += 1;
            }
            if run.0 != b'?' {
                repeat(&mut out, run);
            }
            out.push('$');
        }
        if k < last {
            out.push('-');
        }
    }
    out.push_str("\x1b\\");
    out
}

/// `count` copies of `sixel`, run-length encoded when that is shorter.
fn repeat(out: &mut String, (sixel, count): (u8, usize)) {
    match count {
        0 => {}
        1..=3 => out.extend(std::iter::repeat_n(sixel as char, count)),
        _ => {
            let _ = write!(out, "!{count}{}", sixel as char);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn one_image_per_row_of_cells() {
        let canvas = RgbaImage::from_fn(8, 12, |x, _| {
            Rgba(if x < 4 {
                [255, 0, 0, 255]
            } else {
                [0, 0, 0, 0]
            })
        });
        let rows = encode(&canvas, 6);
        assert_eq!(rows.len(), 2);
        assert!(rows[0].starts_with("\x1bP0;1;0q\"1;1;8;6#"));
        assert!(rows[0].ends_with("!4~$\x1b\\"), "{}", rows[0]);
        assert_eq!(rows[0], rows[1]);
    }

    #[test]
    fn transparent_images_draw_nothing() {
        assert_eq!(encode(&RgbaImage::new(4, 4), 2), ["", ""]);
    }

    #[test]
    fn bands_split_rows_taller_than_six_pixels() {
        let pixels = vec![0; 2 * 8];
        let out = image(&pixels, 2, &[10, 20, 30]);
        assert_eq!(out, "\x1bP0;1;0q\"1;1;2;8#0;2;3;7;11#0~~$-#0BB$\x1b\\");
    }

    #[test]
    fn runs_are_compressed() {
        let mut out = String::new();
        repeat(&mut out, (b'~', 3));
        repeat(&mut out, (b'A', 5));
        assert_eq!(out, "~~~!5A");
    }
}
