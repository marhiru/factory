//! Keyframes (seek paralelo, um ffmpeg por imagem) e contact sheets com rótulo em fonte bitmap embutida
//! (não depende de drawtext/freetype no ffmpeg).
use crate::util::{ffmpeg, run};
use anyhow::Result;
use image::{imageops, ImageBuffer, Rgb, RgbImage};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Fonte 5x7: cada glifo são 7 linhas de 5 bits.
fn glyph(c: char) -> [u8; 7] {
    match c {
        '0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
        '1' => [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E],
        '2' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F],
        '3' => [0x1F, 0x02, 0x04, 0x02, 0x01, 0x11, 0x0E],
        '4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
        '5' => [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E],
        '6' => [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E],
        '7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        '8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        '9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C],
        ':' => [0x00, 0x0C, 0x0C, 0x00, 0x0C, 0x0C, 0x00],
        '.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C],
        ',' => [0x00, 0x00, 0x00, 0x00, 0x0C, 0x04, 0x08],
        '#' => [0x0A, 0x0A, 0x1F, 0x0A, 0x1F, 0x0A, 0x0A],
        '-' => [0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00],
        '(' => [0x02, 0x04, 0x08, 0x08, 0x08, 0x04, 0x02],
        ')' => [0x08, 0x04, 0x02, 0x02, 0x02, 0x04, 0x08],
        '/' => [0x01, 0x01, 0x02, 0x04, 0x08, 0x10, 0x10],
        's' => [0x00, 0x00, 0x0E, 0x10, 0x0E, 0x01, 0x1E],
        'f' => [0x06, 0x09, 0x08, 0x1C, 0x08, 0x08, 0x08],
        'x' => [0x00, 0x00, 0x11, 0x0A, 0x04, 0x0A, 0x11],
        'C' => [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E],
        'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'T' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        _ => [0; 7],
    }
}

pub fn draw_text(img: &mut RgbImage, x0: u32, y0: u32, text: &str, scale: u32, fg: [u8; 3], bg: [u8; 3]) {
    let w = text.chars().count() as u32 * 6 * scale + scale;
    let h = 9 * scale;
    for y in y0..(y0 + h).min(img.height()) {
        for x in x0..(x0 + w).min(img.width()) {
            img.put_pixel(x, y, Rgb(bg));
        }
    }
    for (k, c) in text.chars().enumerate() {
        let g = glyph(c);
        for (row, bits) in g.iter().enumerate() {
            for col in 0..5 {
                if bits >> (4 - col) & 1 == 1 {
                    for dy in 0..scale {
                        for dx in 0..scale {
                            let x = x0 + scale + (k as u32 * 6 + col) * scale + dx;
                            let y = y0 + scale + row as u32 * scale + dy;
                            if x < img.width() && y < img.height() {
                                img.put_pixel(x, y, Rgb(fg));
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Extrai um JPEG por tempo, em paralelo. Lado maior limitado a `max_side`.
pub fn extract_keyframes(input: &Path, times: &[f64], out_dir: &Path, prefix: &str, max_side: u32, hwaccel: bool) -> Result<Vec<PathBuf>> {
    let scale = format!("scale='if(gt(iw,ih),min({m},iw),-2)':'if(gt(iw,ih),-2,min({m},ih))'", m = max_side);
    times
        .par_iter()
        .enumerate()
        .map(|(k, &t)| {
            let p = out_dir.join(format!("{prefix}{:04}.jpg", k + 1));
            let mut cmd = Command::new(ffmpeg());
            cmd.args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y"]);
            if hwaccel {
                cmd.args(["-hwaccel", "videotoolbox"]);
            }
            cmd.args(["-ss", &format!("{t:.4}")]).arg("-i").arg(input);
            cmd.args(["-frames:v", "1", "-vf", &scale, "-q:v", "3"]).arg(&p);
            run(&mut cmd)?;
            Ok(p)
        })
        .collect()
}

pub struct Tile {
    pub img: RgbImage,
    pub label: String,
}

/// Monta sheets em grade (cols x rows) e salva `<prefix>NN.jpg`.
pub fn contact_sheets(tiles: Vec<Tile>, cols: usize, rows: usize, thumb_w: u32, out_prefix: &Path) -> Result<Vec<PathBuf>> {
    if tiles.is_empty() {
        return Ok(vec![]);
    }
    let per = cols * rows;
    let th = {
        let t = &tiles[0].img;
        ((thumb_w as f64) * t.height() as f64 / t.width() as f64).round() as u32
    };
    let gap = 4u32;
    let scale = (thumb_w / 110).clamp(1, 4);
    let chunks: Vec<Vec<Tile>> = {
        let mut v = Vec::new();
        let mut it = tiles.into_iter().peekable();
        while it.peek().is_some() {
            v.push(it.by_ref().take(per).collect());
        }
        v
    };
    chunks
        .into_par_iter()
        .enumerate()
        .map(|(si, chunk)| {
            let r = chunk.len().div_ceil(cols) as u32;
            let c = cols.min(chunk.len()) as u32;
            let mut sheet: RgbImage = ImageBuffer::from_pixel(c * (thumb_w + gap) + gap, r * (th + gap) + gap, Rgb([20, 20, 20]));
            for (k, t) in chunk.iter().enumerate() {
                let im = imageops::resize(&t.img, thumb_w, th, imageops::FilterType::Triangle);
                let x = gap + (k as u32 % c) * (thumb_w + gap);
                let y = gap + (k as u32 / c) * (th + gap);
                imageops::replace(&mut sheet, &im, x as i64, y as i64);
                draw_text(&mut sheet, x, y, &t.label, scale, [255, 235, 80], [0, 0, 0]);
            }
            let p = PathBuf::from(format!("{}{:02}.jpg", out_prefix.display(), si + 1));
            let mut f = std::io::BufWriter::new(std::fs::File::create(&p)?);
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut f, 85).encode_image(&sheet)?;
            Ok(p)
        })
        .collect()
}

pub fn rgb_to_image(w: usize, h: usize, rgb: Vec<u8>) -> RgbImage {
    ImageBuffer::from_raw(w as u32, h as u32, rgb).expect("tamanho de buffer")
}
