use std::{f64::consts::PI, path::Path};

use anyhow::Result;
use image::{ImageBuffer, ImageReader, Luma, Rgb};
use once_cell::sync::Lazy;

type Block = [f64; 8 * 8];

fn calc_dct(src: &[f64], dst: &mut [f64]) {
    const K: f64 = PI / 16.0;
    static A0: Lazy<f64> = Lazy::<f64>::new(|| 1.0 / 2.0f64.sqrt());

    for di in 0..8 {
        let mut acc = 0.0;
        for si in 0..8 {
            acc += src[si] / 2.0 * ((2.0 * si as f64 + 1.0) * K * (di as f64)).cos();
        }
        if di == 0 {
            dst[di] = *A0 / 2.0 * acc;
        } else {
            dst[di] = acc / 2.0;
        }
    }
}

fn rgb2yuv(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let r = r as f64;
    let g = g as f64;
    let b = b as f64;

    let y = 0.299 * r + 0.587 * g + 0.114 * b;
    let u = 0.5 * (b - y) / (1.0 - 0.114) + 128.0;
    let v = 0.5 * (r - y) / (1.0 - 0.299) + 128.0;

    return (y, u, v);
}

fn yuv2rgb(y: f64, u: f64, v: f64) -> (u8, u8, u8) {
    let r = y + 1.402 * (v - 128.0);
    let g = y - (0.114 * 1.772 * (u - 128.0) + 0.299 * 1.402 * (v - 128.0)) / 0.587;
    let b = y + 1.772 * (u - 128.0);
    return (
        r.clamp(0.0, 255.0) as u8,
        g.clamp(0.0, 255.0) as u8,
        b.clamp(0.0, 255.0) as u8,
    );
}

fn loadPlanes<P: AsRef<Path>>(filename: P, yp: &mut Vec<f64>, up: &mut Vec<f64>, vp: &mut Vec<f64>) -> Result<()> {
    let img = ImageReader::open(filename)?.decode()?.to_rgb8();

    let image_width = img.width();
    let image_height = img.height();
    let y_plane_width = (image_width as f64 / 16.0).ceil() as u32 * 16;
    let y_plane_height = (image_height as f64 / 16.0).ceil() as u32 * 16;
    let uv_plane_width = y_plane_width / 2;
    let uv_plane_height = y_plane_height / 2;

    for py in 0..y_plane_height {
        for px in 0..y_plane_width {
            let ix = px.min(image_width - 1);
            let iy = py.min(image_height - 1);
            let Rgb([r, g, b]) = *img.get_pixel(ix, iy);

            let y_index = (px + py * y_plane_width) as usize;
            let uv_index = (px / 2 + py / 2 * uv_plane_width) as usize;
            let (y, u, v) = rgb2yuv(r, g, b);
            yp[y_index] = y;
            up[uv_index] += u;
            vp[uv_index] += v;
        }
    }

    for p in up.iter_mut() {
        *p /= 4.0;
    }
    for p in vp.iter_mut() {
        *p /= 4.0;
    }

    return Ok(());
}

fn main() -> Result<()> {
    let img = ImageReader::open("data/vid/test10/001.tif")?.decode()?.to_rgb8();

    let image_width = img.width();
    let image_height = img.height();
    let y_plane_width = (image_width as f64 / 16.0).ceil() as u32 * 16;
    let y_plane_height = (image_height as f64 / 16.0).ceil() as u32 * 16;
    let uv_plane_width = y_plane_width / 2;
    let uv_plane_height = y_plane_height / 2;

    println!(
        "    image: {}x{}\n  Y plane: {}x{}\nUV planes: {}x{}",
        image_width, image_height, y_plane_width, y_plane_height, uv_plane_width, uv_plane_height
    );

    let mut y_plane: Vec<f64> = vec![0.0; (y_plane_width * y_plane_height) as usize];
    let mut u_plane: Vec<f64> = vec![0.0; (uv_plane_width * uv_plane_height) as usize];
    let mut v_plane: Vec<f64> = vec![0.0; (uv_plane_width * uv_plane_height) as usize];

    let mut y_plane2: Vec<f64> = vec![0.0; (y_plane_width * y_plane_height) as usize];
    let mut u_plane2: Vec<f64> = vec![0.0; (uv_plane_width * uv_plane_height) as usize];
    let mut v_plane2: Vec<f64> = vec![0.0; (uv_plane_width * uv_plane_height) as usize];

    loadPlanes("data/vid/test10/001.tif", &mut y_plane, &mut u_plane, &mut v_plane)?;
    loadPlanes("data/vid/test10/002.tif", &mut y_plane2, &mut u_plane2, &mut v_plane2)?;

    y_plane.iter_mut().zip(y_plane2.iter()).for_each(|(x, y)| *x = *x - y);
    let min_val = y_plane.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max_val = y_plane.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

    println!("{} {}", min_val, max_val);

    let mut result_diff = ImageBuffer::new(y_plane_width, y_plane_height);
    for py in 0..y_plane_height {
        for px in 0..y_plane_width {
            let y_index = (px + py * y_plane_width) as usize;

            let y = y_plane[y_index];
            let r = if y >= 0.0 {
                (y / max_val * 255.0).min(255.0) as u8
            } else {
                0
            };
            let g = if y < 0.0 {
                (y / min_val * 255.0).min(255.0) as u8
            } else {
                0
            };

            result_diff.put_pixel(px, py, Rgb([r, g, 0]));
        }
    }

    result_diff.save("data/result_diff.png")?;

    /*let mut result_y = ImageBuffer::new(y_plane_width, y_plane_height);
    let mut result_u = ImageBuffer::new(uv_plane_width, uv_plane_height);
    let mut result_v = ImageBuffer::new(uv_plane_width, uv_plane_height);
    let mut result_full = ImageBuffer::new(image_width, image_height);

    for py in 0..y_plane_height {
        for px in 0..y_plane_width {
            let y_index = (px + py * y_plane_width) as usize;
            let uv_index = (px / 2 + py / 2 * uv_plane_width) as usize;

            let y = y_plane[y_index];
            let u = u_plane[uv_index];
            let v = v_plane[uv_index];
            let (r, g, b) = yuv2rgb(y, u, v);

            result_y.put_pixel(px, py, Luma([y as u8]));
            result_u.put_pixel(px / 2, py / 2, Luma([u as u8]));
            result_v.put_pixel(px / 2, py / 2, Luma([v as u8]));
            if px < image_width && py < image_height {
                result_full.put_pixel(px, py, Rgb([r, g, b]));
            }
        }
    }

    result_y.save("data/result_y.png")?;
    result_u.save("data/result_u.png")?;
    result_v.save("data/result_v.png")?;
    result_full.save("data/result.png")?;*/

    Ok(())
}
