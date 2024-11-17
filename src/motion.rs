use std::cmp::{max, min};

use crate::{
    blocks::QMatrices,
    planes::Plane,
    videocode::{MacroBlock, VideoFrame},
};

#[derive(Clone, Copy)]
pub enum BlockType {
    New,
    Motion(i32, i32),
}

pub struct MotionMap {
    pub vectors: Vec<BlockType>,
    pub width: u32,
    pub height: u32,
}

const ZMP_TRESHOLD: f64 = 512.0;
const NEW_TRESHOLD: f64 = 4096.0;

fn block_diff(a: &Plane, ax: u32, ay: u32, b: &Plane, bx: u32, by: u32) -> f64 {
    let mut accum = 0f64;
    for y in 0..16 {
        let astart = (ax + (ay + y) * a.width()) as usize;
        let bstart = (bx + (by + y) * b.width()) as usize;
        let aline = &a.data[astart..astart + 16];
        let bline = &b.data[bstart..bstart + 16];
        accum += aline
            .iter()
            .zip(bline.iter())
            .map(|(a, b)| (*a - *b).abs())
            .sum::<f64>();
    }
    return accum;
}

pub fn block_stat(a: &Plane, ax: u32, ay: u32, b: &Plane, bx: u32, by: u32) -> (f64, f64) {
    let mut accum = 0f64;
    let mut accum_sq = 0f64;
    for y in 0..16 {
        let astart = (ax + (ay + y) * a.width()) as usize;
        let bstart = (bx + (by + y) * b.width()) as usize;
        let aline = &a.data[astart..astart + 16];
        let bline = &b.data[bstart..bstart + 16];
        accum += aline
            .iter()
            .zip(bline.iter())
            .map(|(a, b)| (*a - *b).abs())
            .sum::<f64>();
        accum_sq += aline
            .iter()
            .zip(bline.iter())
            .map(|(a, b)| (*a - *b) * (*a - *b))
            .sum::<f64>();
    }
    return (accum, accum_sq);
}

fn block_diff_ult(a: &VideoFrame, ax: u32, ay: u32, b: &VideoFrame, bx: u32, by: u32, qmatrices: &QMatrices) -> usize {
    let mut block_a = MacroBlock::new();
    let mut block_b = MacroBlock::new();

    a.extract_macroblock(ax, ay, &mut block_a);
    b.extract_macroblock(bx, by, &mut block_b);
    block_a.difference(&block_b);
    return block_a.get_encoded_size(qmatrices);
}

impl MotionMap {
    pub fn new(frame: &VideoFrame) -> MotionMap {
        let width = frame.width / 16;
        let height = frame.height / 16;
        return MotionMap {
            vectors: vec![BlockType::New; (width * height) as usize],
            width,
            height,
        };
    }

    pub fn calculate(&mut self, cur_frame: &VideoFrame, prev_frame: &VideoFrame) {
        for my in 0..self.height {
            for mx in 0..self.width {
                let mv_index = (mx + my * self.width) as usize;
                let dst_x = mx * 16;
                let dst_y = my * 16;

                let mut vect = (0i32, 0i32);
                let mut min_d = block_diff(&cur_frame.y_plane, dst_x, dst_y, &prev_frame.y_plane, dst_x, dst_y);
                if min_d > ZMP_TRESHOLD {
                    for by in max(dst_y as i32 - 7, 0)..=min(dst_y as i32 + 7, prev_frame.height as i32 - 1 - 16) {
                        for bx in max(dst_x as i32 - 7, 0)..=min(dst_x as i32 + 7, prev_frame.width as i32 - 1 - 16) {
                            let new_d = block_diff(
                                &cur_frame.y_plane,
                                dst_x,
                                dst_y,
                                &prev_frame.y_plane,
                                bx as u32,
                                by as u32,
                            );
                            if new_d < min_d {
                                min_d = new_d;
                                vect = (bx - dst_x as i32, by - dst_y as i32);
                            }
                        }
                    }
                    if min_d > NEW_TRESHOLD {
                        self.vectors[mv_index] = BlockType::New;
                    } else {
                        self.vectors[mv_index] = BlockType::Motion(vect.0, vect.1);
                    }
                } else {
                    self.vectors[mv_index] = BlockType::Motion(0, 0);
                }
            }
        }
    }

    pub fn calculate_ult(&mut self, cur_frame: &VideoFrame, prev_frame: &VideoFrame, qmatrices: &QMatrices) {
        let mut total = 0usize;
        for my in 0..self.height {
            for mx in 0..self.width {
                let mv_index = (mx + my * self.width) as usize;
                let dst_x = mx * 16;
                let dst_y = my * 16;

                let mut vect = BlockType::New;
                let mut temp = MacroBlock::new();
                cur_frame.extract_macroblock(dst_x, dst_y, &mut temp);
                let mut min_d = temp.get_encoded_size(qmatrices);
                for by in max(dst_y as i32 - 7, 0)..=min(dst_y as i32 + 7, prev_frame.height as i32 - 1 - 16) {
                    for bx in max(dst_x as i32 - 7, 0)..=min(dst_x as i32 + 7, prev_frame.width as i32 - 1 - 16) {
                        let new_d =
                            block_diff_ult(&cur_frame, dst_x, dst_y, &prev_frame, bx as u32, by as u32, qmatrices);
                        if new_d < min_d {
                            min_d = new_d;
                            vect = BlockType::Motion(bx - dst_x as i32, by - dst_y as i32);
                        }
                    }
                }
                total += min_d;
                self.vectors[mv_index] = vect;
            }
        }
        print!("total: {}", total);
    }
}
