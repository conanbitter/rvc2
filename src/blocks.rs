use std::{f64::consts::PI, fmt};

use anyhow::Result;
use once_cell::sync::Lazy;

use crate::bitio::{BitReader, BitWriter};

#[derive(Clone)]
pub struct Block(pub [f64; 8 * 8]);

const QMATRIX_LUMA: [f64; 8 * 8] = [
    16.0, 11.0, 10.0, 16.0, 24.0, 40.0, 51.0, 61.0, //
    12.0, 12.0, 14.0, 19.0, 26.0, 58.0, 60.0, 55.0, //
    14.0, 13.0, 16.0, 24.0, 40.0, 57.0, 69.0, 56.0, //
    14.0, 17.0, 22.0, 29.0, 51.0, 87.0, 80.0, 62.0, //
    18.0, 22.0, 37.0, 56.0, 68.0, 109.0, 103.0, 77.0, //
    24.0, 35.0, 55.0, 64.0, 81.0, 104.0, 113.0, 92.0, //
    49.0, 64.0, 78.0, 87.0, 103.0, 121.0, 120.0, 101.0, //
    72.0, 92.0, 95.0, 98.0, 112.0, 100.0, 103.0, 99.0,
];

const UNWRAP_PATTERN: [usize; 8 * 8] = [
    0, 1, 8, 16, 9, 2, 3, 10, //
    17, 24, 32, 25, 18, 11, 4, 5, //
    12, 19, 26, 33, 40, 48, 41, 34, //
    27, 20, 13, 6, 7, 14, 21, 28, //
    35, 42, 49, 56, 57, 50, 43, 36, //
    29, 22, 15, 23, 30, 37, 44, 51, //
    58, 59, 52, 45, 38, 31, 39, 46, //
    53, 60, 61, 54, 47, 55, 62, 63,
];

const WRAP_PATTERN: [usize; 8 * 8] = [
    0, 1, 5, 6, 14, 15, 27, 28, //
    2, 4, 7, 13, 16, 26, 29, 42, //
    3, 8, 12, 17, 25, 30, 41, 43, //
    9, 11, 18, 24, 31, 40, 44, 53, //
    10, 19, 23, 32, 39, 45, 52, 54, //
    20, 22, 33, 38, 46, 51, 55, 60, //
    21, 34, 37, 47, 50, 56, 59, 61, //
    35, 36, 48, 49, 57, 58, 62, 63,
];

const HUFFMAN_ENCODE: [[i8; 16]; 256] = [
    [1, 0, 1, 0, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [0, 0, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [0, 1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [1, 0, 0, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [1, 0, 1, 1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 0, 1, 0, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 0, 0, 0, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 0, 0, 0, -1, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 1, 0, 1, 1, 0, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 1, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 1, 1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 0, 0, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 0, 1, 1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 0, 0, 1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 0, 1, 1, 0, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 0, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 1, 0, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 1, 0, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 0, 0, 0],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 0, 0, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 0, 0, 1, -1, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 1, 0, 1, 1, 1, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 0, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 0, 0, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 0, 1, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 0, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1, 0, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1, 0, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1, 0],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 0, 1, 0, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 0, 1, 1, 1, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 0, 0, 0, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 0, 0, 0, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 0, 0, 1, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 0, 0, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 0, 1, 0, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 0, 1, 0, 1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 0, 1, 1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 1, 1, 0, 0, 0, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 0, 1, 1, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 0, 1, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 1, 0, 0, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 1, 0, 0, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 1, 0, 1, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 1, 0, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 1, 1, 0, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 1, 1, 0, 1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 0, 1, 0, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 1, 1, 1, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 1, 1, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 0, 0, 0, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 0, 0, 0, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 0, 0, 1, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 0, 0, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 0, 1, 0, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 0, 1, 0, 1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 0, 1, 1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 0, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 0, 1, 1, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 0, 1, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1, 0, 0, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1, 0, 0, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1, 0, 1, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1, 0, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1, 1, 0, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1, 1, 0, 1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 0, 1, 0, -1, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1, 1, 1, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1, 1, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 0, 0, 0, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 0, 0, 0, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 0, 0, 1, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 0, 0, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 0, 1, 0, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 0, 1, 0, 1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 1, 0, 0, 0, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, -1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 0, 1, 1, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 0, 1, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 0, 0, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 0, 0, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 0, 1, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 0, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 0, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 0, 1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 1, 0, 0, 1, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 1, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 0, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 0, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1, 0],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 1, 0, 1, 0, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 0, 0, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 0, 0, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 0, 1, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 0, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 1, 0, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 1, 0, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 1, 1, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 1, 1, 1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 1, 1, 0, 0, 1, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 0, 0, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 0, 0, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 0, 1, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 0, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1, 0, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1, 0, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1, 1, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 0, 0, 0],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 1, 1, 0, 1, 0, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 0, 0, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 0, 1, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 0, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 0, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 0, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 0, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 0, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 1, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 0, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 0, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1, 0],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 0, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 0, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 0],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, -1, -1, -1, -1, -1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
];

const HUFFMAN_DECODE: [[i16; 2]; 162] = [
    [1, 2],
    [-1, -2],
    [1, 2],
    [-3, 2],
    [2, 3],
    [0, -4],
    [-17, 2],
    [2, 3],
    [-5, -18],
    [-33, 2],
    [2, 3],
    [-49, -65],
    [2, 3],
    [3, 4],
    [-6, -19],
    [-81, -97],
    [2, 3],
    [3, 4],
    [-7, -34],
    [-113, 3],
    [3, 4],
    [4, 5],
    [-20, -50],
    [-129, -145],
    [-161, 3],
    [3, 4],
    [4, 5],
    [-8, -35],
    [-66, -177],
    [-193, 3],
    [3, 4],
    [4, 5],
    [-21, -82],
    [-209, -240],
    [3, 4],
    [4, 5],
    [5, 6],
    [-36, -51],
    [-98, -114],
    [4, 5],
    [5, 6],
    [6, 7],
    [7, 8],
    [8, 9],
    [9, 10],
    [10, 11],
    [11, 12],
    [12, 13],
    [13, 14],
    [14, 15],
    [15, 16],
    [16, 17],
    [17, 18],
    [18, 19],
    [19, 20],
    [20, 21],
    [21, 22],
    [22, 23],
    [23, 24],
    [24, 25],
    [25, 26],
    [26, 27],
    [27, 28],
    [28, 29],
    [29, 30],
    [30, 31],
    [31, 32],
    [-130, 32],
    [32, 33],
    [33, 34],
    [34, 35],
    [35, 36],
    [36, 37],
    [37, 38],
    [38, 39],
    [39, 40],
    [40, 41],
    [41, 42],
    [42, 43],
    [43, 44],
    [44, 45],
    [45, 46],
    [46, 47],
    [47, 48],
    [48, 49],
    [49, 50],
    [50, 51],
    [51, 52],
    [52, 53],
    [53, 54],
    [54, 55],
    [55, 56],
    [56, 57],
    [57, 58],
    [58, 59],
    [59, 60],
    [60, 61],
    [61, 62],
    [62, 63],
    [-9, -10],
    [-22, -23],
    [-24, -25],
    [-26, -37],
    [-38, -39],
    [-40, -41],
    [-42, -52],
    [-53, -54],
    [-55, -56],
    [-57, -58],
    [-67, -68],
    [-69, -70],
    [-71, -72],
    [-73, -74],
    [-83, -84],
    [-85, -86],
    [-87, -88],
    [-89, -90],
    [-99, -100],
    [-101, -102],
    [-103, -104],
    [-105, -106],
    [-115, -116],
    [-117, -118],
    [-119, -120],
    [-121, -122],
    [-131, -132],
    [-133, -134],
    [-135, -136],
    [-137, -138],
    [-146, -147],
    [-148, -149],
    [-150, -151],
    [-152, -153],
    [-154, -162],
    [-163, -164],
    [-165, -166],
    [-167, -168],
    [-169, -170],
    [-178, -179],
    [-180, -181],
    [-182, -183],
    [-184, -185],
    [-186, -194],
    [-195, -196],
    [-197, -198],
    [-199, -200],
    [-201, -202],
    [-210, -211],
    [-212, -213],
    [-214, -215],
    [-216, -217],
    [-218, -225],
    [-226, -227],
    [-228, -229],
    [-230, -231],
    [-232, -233],
    [-234, -241],
    [-242, -243],
    [-244, -245],
    [-246, -247],
    [-248, -249],
    [-250, 0],
];

type DctMatrix = [[f64; 8 * 8]; 8 * 8];

static DCT_K: Lazy<DctMatrix> = Lazy::<DctMatrix>::new(|| {
    let mut result = [[0f64; 8 * 8]; 8 * 8];

    for v in 0..8 {
        for u in 0..8 {
            let uv = u + v * 8;
            let au = if u == 0 { 1.0 / 2.0f64.sqrt() } else { 1.0 };
            let av = if v == 0 { 1.0 / 2.0f64.sqrt() } else { 1.0 };
            let pred = au * av / 4.0;
            for y in 0..8 {
                for x in 0..8 {
                    let xy = x + y * 8;
                    result[uv][xy] = pred
                        * ((2.0 * x as f64 + 1.0) * u as f64 * PI / 16.0).cos()
                        * ((2.0 * y as f64 + 1.0) * v as f64 * PI / 16.0).cos();
                }
            }
        }
    }

    result
});

static UNDCT_K: Lazy<DctMatrix> = Lazy::<DctMatrix>::new(|| {
    let mut result = [[0f64; 8 * 8]; 8 * 8];

    for x in 0..8 {
        for y in 0..8 {
            let xy = x + y * 8;
            for u in 0..8 {
                for v in 0..8 {
                    let uv = u + v * 8;
                    let au = if u == 0 { 1.0 / 2.0f64.sqrt() } else { 1.0 };
                    let av = if v == 0 { 1.0 / 2.0f64.sqrt() } else { 1.0 };
                    let pred = au * av / 4.0;
                    result[xy][uv] = pred
                        * ((2.0 * x as f64 + 1.0) * u as f64 * PI / 16.0).cos()
                        * ((2.0 * y as f64 + 1.0) * v as f64 * PI / 16.0).cos();
                }
            }
        }
    }

    result
});

impl fmt::Debug for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[ ")?;
        for y in 0..8 {
            for x in 0..8 {
                write!(f, "{:7.1}, ", self.0[x + y * 8])?;
            }
            if y < 7 {
                write!(f, "\n  ")?;
            }
        }
        write!(f, " ]")?;
        return Ok(());
    }
}

impl Block {
    pub fn new() -> Block {
        Block([0.0; 8 * 8])
    }

    pub fn apply_dct(&self, dst: &mut Block) {
        for (uv, d) in dst.0.iter_mut().enumerate() {
            *d = self.0.iter().enumerate().map(|(xy, g)| g * DCT_K[uv][xy]).sum();
        }
    }

    pub fn revert_dct(&self, dst: &mut Block) {
        for (xy, d) in dst.0.iter_mut().enumerate() {
            *d = self.0.iter().enumerate().map(|(uv, g)| g * UNDCT_K[xy][uv]).sum();
        }
    }

    pub fn quantization(&mut self) {
        for (d, q) in self.0.iter_mut().zip(QMATRIX_LUMA) {
            *d = (*d / q).round();
        }
    }

    pub fn dequantization(&mut self) {
        for (d, q) in self.0.iter_mut().zip(QMATRIX_LUMA) {
            *d = *d * q;
        }
    }

    pub fn unwrap(&self, dst: &mut Block) {
        for (i, d) in dst.0.iter_mut().enumerate() {
            *d = self.0[UNWRAP_PATTERN[i]];
        }
    }

    pub fn wrap(&self, dst: &mut Block) {
        /*let mut wrap_pattern = [0usize; 8 * 8];
        for i in 0..64 {
            wrap_pattern[UNWRAP_PATTERN[i]] = i;
        }
        println!("{:?}", wrap_pattern);*/
        for (i, d) in dst.0.iter_mut().enumerate() {
            *d = self.0[WRAP_PATTERN[i]];
        }
    }

    pub fn int_width(value: i16) -> usize {
        if value <= -1024 || value >= 1024 {
            return 11;
        }
        if value <= -512 || value >= 512 {
            return 10;
        }
        if value <= -256 || value >= 256 {
            return 9;
        }
        if value <= -128 || value >= 128 {
            return 8;
        }
        if value <= -64 || value >= 64 {
            return 7;
        }
        if value <= -32 || value >= 32 {
            return 6;
        }
        if value <= -16 || value >= 16 {
            return 5;
        }
        if value <= -8 || value >= 8 {
            return 4;
        }
        if value <= -4 || value >= 4 {
            return 3;
        }
        if value <= -2 || value >= 2 {
            return 2;
        }
        if value <= -1 || value >= 1 {
            return 1;
        }
        return 0;
    }

    pub fn encode(&self, writer: &mut BitWriter) -> Result<()> {
        let dc = self.0[0] as i16;
        writer.write_vec(&HUFFMAN_ENCODE[Block::int_width(dc) as usize])?;
        writer.write_varint(dc)?;
        let mut zeroes = 0;
        let mut tail = 0;
        for i in 0..8 * 8 {
            if self.0[63 - i] as i16 != 0 {
                break;
            }
            tail += 1;
        }
        for i in 1..64 - tail {
            let item = self.0[i] as i16;
            if item == 0 {
                zeroes += 1;
                if zeroes == 16 {
                    writer.write_vec(&HUFFMAN_ENCODE[0xF0])?;
                    zeroes = 0;
                }
            } else {
                let item_width = Block::int_width(item);
                let head = (zeroes as u8) << 4 | item_width as u8;
                writer.write_vec(&HUFFMAN_ENCODE[head as usize])?;
                writer.write_varint(item)?;
                zeroes = 0;
            }
        }
        if tail > 0 {
            writer.write_vec(&HUFFMAN_ENCODE[0x00])?;
        }
        return Ok(());
    }

    fn process_pixel(&self, index: usize) -> i16 {
        let unwrapped_index = UNWRAP_PATTERN[index];
        // DCT
        let pixel: f64 = self
            .0
            .iter()
            .enumerate()
            .map(|(xy, g)| g * DCT_K[unwrapped_index][xy])
            .sum();
        // Quantization
        let pixel = (pixel / QMATRIX_LUMA[unwrapped_index]).round();
        return pixel as i16;
    }

    pub fn encode2(&self, writer: &mut BitWriter) -> Result<()> {
        let mut temp = [0i16; 8 * 8];
        // Convert block
        for (i, d) in temp.iter_mut().enumerate() {
            let unwrapped_index = UNWRAP_PATTERN[i];
            // DCT
            let pixel: f64 = self
                .0
                .iter()
                .enumerate()
                .map(|(xy, g)| g * DCT_K[unwrapped_index][xy])
                .sum();
            // Quantization
            let pixel = (pixel / QMATRIX_LUMA[unwrapped_index]).round();
            *d = pixel as i16;
        }

        let dc = temp[0];
        writer.write_vec(&HUFFMAN_ENCODE[Block::int_width(dc) as usize])?;
        writer.write_varint(dc)?;
        let mut zeroes = 0;
        let mut tail = 0;
        for i in 0..8 * 8 {
            if temp[63 - i] != 0 {
                break;
            }
            tail += 1;
        }
        for i in 1..64 - tail {
            let item = temp[i];
            if item == 0 {
                zeroes += 1;
                if zeroes == 16 {
                    writer.write_vec(&HUFFMAN_ENCODE[0xF0])?;
                    zeroes = 0;
                }
            } else {
                let item_width = Block::int_width(item);
                let head = (zeroes as u8) << 4 | item_width as u8;
                writer.write_vec(&HUFFMAN_ENCODE[head as usize])?;
                writer.write_varint(item)?;
                zeroes = 0;
            }
        }
        if tail > 0 {
            writer.write_vec(&HUFFMAN_ENCODE[0x00])?;
        }
        return Ok(());
    }

    pub fn decode(&mut self, reader: &mut BitReader) -> Result<()> {
        for d in self.0.iter_mut() {
            *d = 0.0;
        }

        let dc_width = reader.decode_huffman(&HUFFMAN_DECODE)?;
        let dc = reader.read_varint(dc_width)?;
        //println!("{} {}", dc_width, dc);

        self.0[0] = dc as f64;

        let mut i = 1;

        while i < 64 {
            let head = reader.decode_huffman(&HUFFMAN_DECODE)?;
            if head == 0xF0 {
                i += 16;
            } else if head == 0x00 {
                break;
            } else {
                let item_width = head & 0b1111;
                let zeroes = head >> 4;
                i += zeroes;
                self.0[i as usize] = reader.read_varint(item_width)? as f64;
                i += 1;
            }
        }

        return Ok(());
    }

    pub fn decode2(&mut self, reader: &mut BitReader) -> Result<()> {
        let mut temp = [0f64; 8 * 8];

        let dc_width = reader.decode_huffman(&HUFFMAN_DECODE)?;
        let dc = reader.read_varint(dc_width)?;

        temp[0] = dc as f64;

        let mut i = 1usize;

        while i < 64 {
            let head = reader.decode_huffman(&HUFFMAN_DECODE)?;
            if head == 0xF0 {
                i += 16;
            } else if head == 0x00 {
                break;
            } else {
                let item_width = head & 0b1111;
                let zeroes = head >> 4;
                i += zeroes as usize;
                temp[i] = reader.read_varint(item_width)? as f64;
                i += 1;
            }
        }

        // Revert
        // Dequantization
        for (i, d) in temp.iter_mut().enumerate() {
            *d *= QMATRIX_LUMA[UNWRAP_PATTERN[i]];
        }
        /*for (i, d) in self.0.iter_mut().enumerate() {
            *d = temp[WRAP_PATTERN[i]];
        }
        println!("{:?}", self);*/
        for (i, d) in self.0.iter_mut().enumerate() {
            //let wrapped_index = ;
            // Revert DCT
            *d = temp
                .iter()
                .enumerate()
                .map(|(uv, g)| g * UNDCT_K[i][UNWRAP_PATTERN[uv]])
                .sum();
        }

        return Ok(());
    }

    pub fn normalize(&mut self) {
        for d in self.0.iter_mut() {
            *d = *d - 128.0;
        }
    }

    pub fn denormalize(&mut self) {
        for d in self.0.iter_mut() {
            *d = *d + 128.0;
        }
    }
}
