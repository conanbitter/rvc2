use std::{io::Write, path::Path};

use anyhow::Result;
use image::{ImageBuffer, ImageReader, Rgb};

use crate::{
    bitio::{BitReader, BitWriter},
    blocks::{Block, QMatrices},
    colors::{rgb2yuv, yuv2rgb},
    motion::{BlockType, MotionMap},
    planes::Plane,
};

#[repr(u8)]
enum FrameType {
    IFrame,
    PFrame,
    BFrame,
}

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

pub struct Encoder {
    buffer: Vec<u8>,
    data: [u8; 1],
}

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

    pub fn average(&mut self, other: &MacroBlock) {
        for (block, other_block) in self.0.iter_mut().zip(other.0.iter()) {
            for (d, other_d) in block.0.iter_mut().zip(other_block.0.iter()) {
                *d = (*d + other_d) / 2.0;
            }
        }
    }

    pub fn encode(&mut self, qmatrices: &QMatrices) {
        self.0[0].encode3(&qmatrices.luma);
        self.0[1].encode3(&qmatrices.luma);
        self.0[2].encode3(&qmatrices.luma);
        self.0[3].encode3(&qmatrices.luma);
        self.0[4].encode3(&qmatrices.chroma);
        self.0[5].encode3(&qmatrices.chroma);
    }

    pub fn write(&self, writer: &mut BitWriter) -> Result<()> {
        self.0[0].write(writer, true)?;
        self.0[1].write(writer, true)?;
        self.0[2].write(writer, true)?;
        self.0[3].write(writer, true)?;
        self.0[4].write(writer, false)?;
        self.0[5].write(writer, false)?;
        return Ok(());
    }

    pub fn get_encoded_size(&self, qmatrices: &QMatrices) -> usize {
        self.0[0].get_encoded_size(&qmatrices.luma, true)
            + self.0[1].get_encoded_size(&qmatrices.luma, true)
            + self.0[2].get_encoded_size(&qmatrices.luma, true)
            + self.0[3].get_encoded_size(&qmatrices.luma, true)
            + self.0[4].get_encoded_size(&qmatrices.chroma, false)
            + self.0[5].get_encoded_size(&qmatrices.chroma, false)
    }

    pub fn decode(&mut self, qmatrices: &QMatrices) {
        self.0[0].decode3(&qmatrices.luma);
        self.0[1].decode3(&qmatrices.luma);
        self.0[2].decode3(&qmatrices.luma);
        self.0[3].decode3(&qmatrices.luma);
        self.0[4].decode3(&qmatrices.chroma);
        self.0[5].decode3(&qmatrices.chroma);
    }

    pub fn read(&mut self, reader: &mut BitReader) -> Result<()> {
        self.0[0].read(reader, true)?;
        self.0[1].read(reader, true)?;
        self.0[2].read(reader, true)?;
        self.0[3].read(reader, true)?;
        self.0[4].read(reader, false)?;
        self.0[5].read(reader, false)?;
        return Ok(());
    }
}

impl Encoder {
    pub fn new() -> Encoder {
        return Encoder {
            buffer: Vec::<u8>::new(),
            data: [0u8; 1],
        };
    }

    pub fn encode_i_frame(&mut self, frame: &VideoFrame, file: &mut dyn Write, qmatrices: &QMatrices) -> Result<()> {
        self.data[0] = FrameType::IFrame as u8;
        file.write_all(&self.data)?;

        self.buffer.clear();
        let mut writer = BitWriter::new(&mut self.buffer);
        let mv_width = frame.width / 16;
        let mv_height = frame.height / 16;
        let mut mblock = MacroBlock::new();

        for my in 0..mv_height {
            for mx in 0..mv_width {
                frame.extract_macroblock(mx * 16, my * 16, &mut mblock);
                mblock.encode(qmatrices);
                mblock.write(&mut writer)?;
            }
        }
        writer.flush()?;
        let dct_size = self.buffer.len() as u32;
        file.write_all(&dct_size.to_ne_bytes())?;
        file.write_all(&self.buffer)?;
        return Ok(());
    }

    pub fn encode_p_frame(
        &mut self,
        frame: &VideoFrame,
        prev_frame: &VideoFrame,
        file: &mut dyn Write,
        qmatrices: &QMatrices,
    ) -> Result<()> {
        self.data[0] = FrameType::PFrame as u8;
        file.write_all(&self.data)?;

        let mut motion = MotionMap::new(&frame);
        motion.calculate(&frame, &prev_frame);
        self.buffer.clear();
        motion.write(&mut self.buffer)?;
        let motion_size = self.buffer.len() as u32;
        file.write_all(&motion_size.to_ne_bytes())?;
        file.write_all(&self.buffer)?;

        self.buffer.clear();
        let mut writer = BitWriter::new(&mut self.buffer);
        let mv_width = frame.width / 16;
        let mv_height = frame.height / 16;
        let mut mblock1 = MacroBlock::new();
        let mut mblock2 = MacroBlock::new();

        for my in 0..mv_height {
            for mx in 0..mv_width {
                let dst_x = mx * 16;
                let dst_y = my * 16;
                let mv_index = (mx + my * mv_width) as usize;

                frame.extract_macroblock(dst_x, dst_y, &mut mblock1);

                if let BlockType::Motion(vx, vy) = motion.vectors[mv_index] {
                    prev_frame.extract_macroblock((dst_x as i32 + vx) as u32, (dst_y as i32 + vy) as u32, &mut mblock2);
                    mblock1.difference(&mblock2);
                }

                mblock1.encode(qmatrices);
                mblock1.write(&mut writer)?;
            }
        }
        writer.flush()?;
        let dct_size = self.buffer.len() as u32;
        file.write_all(&dct_size.to_ne_bytes())?;
        file.write_all(&self.buffer)?;
        return Ok(());
    }

    pub fn encode_b_frame(
        &mut self,
        frame: &VideoFrame,
        prev_frame: &VideoFrame,
        next_frame: &VideoFrame,
        file: &mut dyn Write,
        qmatrices: &QMatrices,
    ) -> Result<()> {
        self.data[0] = FrameType::PFrame as u8;
        file.write_all(&self.data)?;

        let mut motion_prev = MotionMap::new(&frame);
        motion_prev.calculate(&frame, &prev_frame);
        self.buffer.clear();
        motion_prev.write(&mut self.buffer)?;
        let motion_size = self.buffer.len() as u32;
        file.write_all(&motion_size.to_ne_bytes())?;
        file.write_all(&self.buffer)?;

        let mut motion_next = MotionMap::new(&frame);
        motion_next.calculate(&frame, &next_frame);
        self.buffer.clear();
        motion_prev.write(&mut self.buffer)?;
        let motion_size = self.buffer.len() as u32;
        file.write_all(&motion_size.to_ne_bytes())?;
        file.write_all(&self.buffer)?;

        self.buffer.clear();
        let mut writer = BitWriter::new(&mut self.buffer);
        let mv_width = frame.width / 16;
        let mv_height = frame.height / 16;
        let mut mblock1 = MacroBlock::new();
        let mut mblock2 = MacroBlock::new();
        let mut mblock3 = MacroBlock::new();

        for my in 0..mv_height {
            for mx in 0..mv_width {
                let dst_x = mx * 16;
                let dst_y = my * 16;
                let mv_index = (mx + my * mv_width) as usize;

                frame.extract_macroblock(dst_x, dst_y, &mut mblock1);

                if let BlockType::Motion(pvx, pvy) = motion_prev.vectors[mv_index] {
                    prev_frame.extract_macroblock(
                        (dst_x as i32 + pvx) as u32,
                        (dst_y as i32 + pvy) as u32,
                        &mut mblock2,
                    );

                    if let BlockType::Motion(nvx, nvy) = motion_prev.vectors[mv_index] {
                        next_frame.extract_macroblock(
                            (dst_x as i32 + nvx) as u32,
                            (dst_y as i32 + nvy) as u32,
                            &mut mblock3,
                        );
                        mblock2.average(&mblock3);
                    }

                    mblock1.difference(&mblock2);
                } else if let BlockType::Motion(nvx, nvy) = motion_prev.vectors[mv_index] {
                    next_frame.extract_macroblock(
                        (dst_x as i32 + nvx) as u32,
                        (dst_y as i32 + nvy) as u32,
                        &mut mblock3,
                    );
                    mblock1.difference(&mblock3);
                }

                mblock1.encode(qmatrices);
                mblock1.write(&mut writer)?;
            }
        }
        writer.flush()?;
        let dct_size = self.buffer.len() as u32;
        file.write_all(&dct_size.to_ne_bytes())?;
        file.write_all(&self.buffer)?;
        return Ok(());
    }
}
