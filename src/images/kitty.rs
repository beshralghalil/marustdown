//! The kitty graphics protocol: an image is sent once, then placed by id, cropped to
//! the rows in view.

use std::fmt::Write as _;

/// Payload bytes per escape sequence, as the protocol requires.
const CHUNK: usize = 4096;

/// Sends `png` (base64) as image `id` without showing it.
pub fn transmit(id: u32, png: &str, out: &mut String) {
    let chunks = png.as_bytes().chunks(CHUNK);
    let last = chunks.len().saturating_sub(1);
    for (k, chunk) in chunks.enumerate() {
        let more = u8::from(k < last);
        let chunk = std::str::from_utf8(chunk).unwrap_or_default();
        if k == 0 {
            let _ = write!(out, "\x1b_Ga=t,f=100,t=d,i={id},q=2,m={more};{chunk}\x1b\\");
        } else {
            let _ = write!(out, "\x1b_Gm={more};{chunk}\x1b\\");
        }
    }
}

/// Shows rows `skip..skip + rows` of image `id`, `cols` cells wide, at the cursor.
pub fn place(
    id: u32,
    placement: u32,
    cell: (u32, u32),
    cols: u16,
    skip: u16,
    rows: u16,
    out: &mut String,
) {
    let (w, h) = (u32::from(cols) * cell.0, u32::from(rows) * cell.1);
    let y = u32::from(skip) * cell.1;
    let _ = write!(
        out,
        "\x1b_Ga=p,i={id},p={placement},x=0,y={y},w={w},h={h},c={cols},r={rows},C=1,q=2\x1b\\"
    );
}

/// Removes one placement of image `id`, keeping the image for later placements.
pub fn unplace(id: u32, placement: u32, out: &mut String) {
    let _ = write!(out, "\x1b_Ga=d,d=i,i={id},p={placement},q=2\x1b\\");
}

/// Removes every placement, keeping the images.
pub fn unplace_all(out: &mut String) {
    out.push_str("\x1b_Ga=d,d=a,q=2\x1b\\");
}

/// Frees image `id` in the terminal.
pub fn delete(id: u32, out: &mut String) {
    let _ = write!(out, "\x1b_Ga=d,d=I,i={id},q=2\x1b\\");
}

/// Frees every image.
pub fn delete_all(out: &mut String) {
    out.push_str("\x1b_Ga=d,d=A,q=2\x1b\\");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_images_are_chunked() {
        let mut out = String::new();
        transmit(7, &"A".repeat(CHUNK + 4), &mut out);
        assert!(out.starts_with("\x1b_Ga=t,f=100,t=d,i=7,q=2,m=1;AAAA"));
        assert!(out.ends_with("\x1b_Gm=0;AAAA\x1b\\"));
        assert_eq!(out.matches("\x1b_G").count(), 2);
    }

    #[test]
    fn placements_crop_to_rows_in_view() {
        let mut out = String::new();
        place(3, 2, (10, 20), 8, 1, 4, &mut out);
        assert_eq!(
            out,
            "\x1b_Ga=p,i=3,p=2,x=0,y=20,w=80,h=80,c=8,r=4,C=1,q=2\x1b\\"
        );
    }
}
