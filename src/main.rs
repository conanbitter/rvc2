use std::f64::consts::PI;

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

fn main() -> Result<()> {
    let img = ImageReader::open("data/test5.png")?.decode()?.to_rgb8();

    let image_width = img.width();
    let image_height = img.height();
    let plane_width = (image_width as f64 / 16.0).ceil() as u32 * 16;
    let plane_height = (image_height as f64 / 16.0).ceil() as u32 * 16;

    println!("{} {} {} {}", image_width, image_height, plane_width, plane_height);

    let mut y_plane: Vec<f64> = vec![0.0; (plane_width * plane_height) as usize];
    let mut u_plane: Vec<f64> = vec![0.0; (plane_width * plane_height) as usize];
    let mut v_plane: Vec<f64> = vec![0.0; (plane_width * plane_height) as usize];

    for py in 0..plane_height {
        for px in 0..plane_width {
            let ix = px.min(image_width - 1);
            let iy = py.min(image_height - 1);
            let Rgb([r, g, b]) = *img.get_pixel(ix, iy);

            let y_index = (px + py * plane_width) as usize;
            let uv_index = (px + py * plane_width) as usize; //px / 2 + py / 2 * plane_width;
            let (y, u, v) = rgb2yuv(r, g, b);
            y_plane[y_index] = y;
            u_plane[uv_index] = u;
            v_plane[uv_index] = v;
        }
    }

    let mut result_y = ImageBuffer::new(plane_width, plane_height);
    let mut result_u = ImageBuffer::new(plane_width, plane_height);
    let mut result_v = ImageBuffer::new(plane_width, plane_height);
    let mut result_full = ImageBuffer::new(image_width, image_height);

    for py in 0..plane_height {
        for px in 0..plane_width {
            let y_index = (px + py * plane_width) as usize;
            let uv_index = (px + py * plane_width) as usize;

            let y = y_plane[y_index];
            let u = u_plane[uv_index];
            let v = v_plane[uv_index];
            let (r, g, b) = yuv2rgb(y, u, v);

            result_y.put_pixel(px, py, Luma([y as u8]));
            result_u.put_pixel(px, py, Luma([u as u8]));
            result_v.put_pixel(px, py, Luma([v as u8]));
            if px < image_width && py < image_height {
                result_full.put_pixel(px, py, Rgb([r, g, b]));
            }
        }
    }

    result_y.save("data/result_y.png")?;
    result_u.save("data/result_u.png")?;
    result_v.save("data/result_v.png")?;
    result_full.save("data/result.png")?;

    Ok(())
}
