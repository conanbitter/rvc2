use std::path::Path;

use anyhow::Result;
use image::{GrayImage, ImageReader, Luma, Rgb, RgbImage};

use crate::blocks::Block;

pub struct Plane {
    pub data: Vec<f64>,
    width: u32,
    height: u32,
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

impl Plane {
    pub fn new(width: u32, height: u32) -> Plane {
        Plane {
            data: vec![0.0; (width * height) as usize],
            width,
            height,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn put(&mut self, x: u32, y: u32, value: f64) {
        self.data[(x + y * self.width) as usize] = value;
    }

    pub fn add(&mut self, x: u32, y: u32, value: f64) {
        self.data[(x + y * self.width) as usize] += value;
    }

    pub fn fill(&mut self, value: f64) {
        for d in self.data.iter_mut() {
            *d = value;
        }
    }

    pub fn scale(&mut self, value: f64) {
        for d in self.data.iter_mut() {
            *d *= value;
        }
    }

    pub fn get(&self, x: u32, y: u32) -> f64 {
        self.data[(x + y * self.width) as usize]
    }

    pub fn image2planes<P: AsRef<Path>>(filename: P, yp: &mut Plane, up: &mut Plane, vp: &mut Plane) -> Result<()> {
        let img = ImageReader::open(filename)?.decode()?.to_rgb8();

        let image_width = img.width();
        let image_height = img.height();
        let plane_width = yp.width;
        let plane_height = yp.height;

        yp.fill(0.0);
        up.fill(0.0);
        vp.fill(0.0);

        for py in 0..plane_height {
            for px in 0..plane_width {
                let ix = px.min(image_width - 1) as u32;
                let iy = py.min(image_height - 1) as u32;
                let Rgb([r, g, b]) = *img.get_pixel(ix, iy);

                let (y, u, v) = rgb2yuv(r, g, b);
                yp.put(px, py, y);
                up.add(px / 2, py / 2, u);
                vp.add(px / 2, py / 2, v);
            }
        }

        up.scale(1.0 / 4.0);
        vp.scale(1.0 / 4.0);

        return Ok(());
    }

    pub fn plane2luma(plane: &Plane, image: &mut GrayImage) {
        for (input, output) in plane.data.iter().zip(image.pixels_mut()) {
            *output = Luma([*input as u8]);
        }
    }

    pub fn planes2image(yp: &Plane, up: &Plane, vp: &Plane, image: &mut RgbImage) {
        for (px, py, pixel) in image.enumerate_pixels_mut() {
            let y = yp.get(px, py);
            let u = up.get(px / 2, py / 2);
            let v = vp.get(px / 2, py / 2);
            let (r, g, b) = yuv2rgb(y, u, v);
            *pixel = Rgb([r, g, b]);
        }
    }

    pub fn extract_block(&self, x: u32, y: u32, block: &mut Block) {
        for i in 0..8 {
            let plane_start = (x + (y + i) * self.width) as usize;
            let block_start = (i * 8) as usize;
            block.0[block_start..block_start + 8].copy_from_slice(&self.data[plane_start..plane_start + 8]);
        }
    }

    pub fn apply_block(&mut self, x: u32, y: u32, block: &Block) {
        for i in 0..8 {
            let plane_start = (x + (y + i) * self.width) as usize;
            let block_start = (i * 8) as usize;
            self.data[plane_start..plane_start + 8].copy_from_slice(&block.0[block_start..block_start + 8]);
        }
    }
}
