use std::path::Path;

use anyhow::Result;
use image::{ImageBuffer, ImageReader, Rgb};

use crate::{
    blocks::Block,
    colors::{rgb2yuv, yuv2rgb},
    planes::Plane,
};

pub struct VideoFrame {
    pub y_plane: Plane,
    pub u_plane: Plane,
    pub v_plane: Plane,

    pub source_width: u32,
    pub source_height: u32,
    pub width: u32,
    pub height: u32,
}

pub struct MacroBlock(pub [Block; 4 + 1 + 1]);

impl VideoFrame {
    pub fn new(width: u32, height: u32) -> VideoFrame {
        let plane_width = (width as f64 / 16.0).ceil() as u32 * 16;
        let plane_height = (height as f64 / 16.0).ceil() as u32 * 16;
        return VideoFrame {
            y_plane: Plane::new(plane_width, plane_height),
            u_plane: Plane::new(plane_width / 2, plane_height / 2),
            v_plane: Plane::new(plane_width / 2, plane_height / 2),
            source_width: width,
            source_height: height,
            width: plane_width,
            height: plane_height,
        };
    }

    pub fn load_from_image<P: AsRef<Path>>(&mut self, filename: P) -> Result<()> {
        let img = ImageReader::open(filename)?.decode()?.to_rgb8();

        let image_width = img.width();
        let image_height = img.height();

        self.u_plane.fill(0.0);
        self.v_plane.fill(0.0);

        for py in 0..self.height {
            for px in 0..self.width {
                let ix = px.min(image_width - 1) as u32;
                let iy = py.min(image_height - 1) as u32;
                let Rgb([r, g, b]) = *img.get_pixel(ix, iy);

                let (y, u, v) = rgb2yuv(r, g, b);
                self.y_plane.put(px, py, y);
                self.u_plane.add(px / 2, py / 2, u);
                self.v_plane.add(px / 2, py / 2, v);
            }
        }

        self.u_plane.scale(1.0 / 4.0);
        self.v_plane.scale(1.0 / 4.0);

        return Ok(());
    }

    pub fn save_to_image<P: AsRef<Path>>(&self, filename: P) -> Result<()> {
        let mut image = ImageBuffer::new(self.source_width, self.source_height);
        for (px, py, pixel) in image.enumerate_pixels_mut() {
            let y = self.y_plane.get(px, py);
            let u = self.u_plane.get(px / 2, py / 2);
            let v = self.v_plane.get(px / 2, py / 2);
            let (r, g, b) = yuv2rgb(y, u, v);
            *pixel = Rgb([r, g, b]);
        }
        image.save(filename)?;
        return Ok(());
    }

    pub fn extract_macroblock(&self, x: u32, y: u32, block: &mut MacroBlock) {
        self.y_plane.extract_block(x, y, &mut block.0[0]);
        self.y_plane.extract_block(x + 8, y, &mut block.0[1]);
        self.y_plane.extract_block(x, y + 8, &mut block.0[2]);
        self.y_plane.extract_block(x + 8, y + 8, &mut block.0[3]);
        self.u_plane.extract_block(x / 2, y / 2, &mut block.0[4]);
        self.v_plane.extract_block(x / 2, y / 2, &mut block.0[5]);
    }

    pub fn apply_macroblock(&mut self, x: u32, y: u32, block: &MacroBlock) {
        self.y_plane.apply_block(x, y, &block.0[0]);
        self.y_plane.apply_block(x + 8, y, &block.0[1]);
        self.y_plane.apply_block(x, y + 8, &block.0[2]);
        self.y_plane.apply_block(x + 8, y + 8, &block.0[3]);
        self.u_plane.apply_block(x / 2, y / 2, &block.0[4]);
        self.v_plane.apply_block(x / 2, y / 2, &block.0[5]);
    }
}

impl MacroBlock {
    pub fn new() -> MacroBlock {
        return MacroBlock(core::array::from_fn(|_| Block::new()));
    }

    pub fn difference(&mut self, other: &MacroBlock) {
        for (block, other_block) in self.0.iter_mut().zip(other.0.iter()) {
            for (d, other_d) in block.0.iter_mut().zip(other_block.0.iter()) {
                *d -= other_d;
            }
        }
    }

    pub fn add(&mut self, other: &MacroBlock) {
        for (block, other_block) in self.0.iter_mut().zip(other.0.iter()) {
            for (d, other_d) in block.0.iter_mut().zip(other_block.0.iter()) {
                *d += other_d;
            }
        }
    }
}
