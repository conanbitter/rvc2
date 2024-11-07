use std::cmp::{max, min};

use crate::{planes::Plane, videocode::VideoFrame};

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

const ZMP_TRESHOLD: f64 = 512.0 * 4.0;

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
                    if min_d > ZMP_TRESHOLD {
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
}
