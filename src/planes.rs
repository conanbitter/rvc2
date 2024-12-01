use std::path::Path;

use anyhow::Result;
use image::{GrayImage, ImageReader, Luma, Rgb, RgbImage};

use crate::blocks::Block;

#[derive(Clone)]
pub struct Plane {
    pub data: Vec<f64>,
    width: u32,
    height: u32,
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

    pub fn plane2luma(plane: &Plane, image: &mut GrayImage) {
        for (input, output) in plane.data.iter().zip(image.pixels_mut()) {
            *output = Luma([*input as u8]);
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
