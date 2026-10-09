//! iTerm2 inline images. The protocol can't crop, so each row of cells is its own small
//! PNG, and a partly visible image shows just the rows in view.

use image::{RgbaImage, imageops};

use super::load::png;
use crate::render::base64;

pub fn encode(canvas: &RgbaImage, cols: u16, cell_h: u32) -> Option<Vec<String>> {
    (0..canvas.height() / cell_h)
        .map(|row| {
            let strip = imageops::crop_imm(canvas, 0, row * cell_h, canvas.width(), cell_h);
            let data = png(&strip.to_image())?;
            let mut out = format!(
                "\x1b]1337;File=inline=1;size={};width={cols};height=1;preserveAspectRatio=0:",
                data.len()
            );
            base64(&data, &mut out);
            out.push('\x07');
            Some(out)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_image_per_row() {
        let rows = encode(&RgbaImage::new(20, 40), 2, 20).unwrap();
        assert_eq!(rows.len(), 2);
        assert!(rows[0].starts_with("\x1b]1337;File=inline=1;size="));
        assert!(rows[0].contains(";width=2;height=1;preserveAspectRatio=0:iVBORw0KGgo"));
        assert!(rows[0].ends_with('\x07'));
    }
}
