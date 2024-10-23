use std::{f64::consts::PI, fmt};

use once_cell::sync::Lazy;

pub struct Block(pub [f64; 8 * 8]);

const QMATRIX: [f64; 8 * 8] = [
    16.0, 11.0, 10.0, 16.0, 24.0, 40.0, 51.0, 61.0, //
    12.0, 12.0, 14.0, 19.0, 26.0, 58.0, 60.0, 55.0, //
    14.0, 13.0, 16.0, 24.0, 40.0, 57.0, 69.0, 56.0, //
    14.0, 17.0, 22.0, 29.0, 51.0, 87.0, 80.0, 62.0, //
    18.0, 22.0, 37.0, 56.0, 68.0, 109.0, 103.0, 77.0, //
    24.0, 35.0, 55.0, 64.0, 81.0, 104.0, 113.0, 92.0, //
    49.0, 64.0, 78.0, 87.0, 103.0, 121.0, 120.0, 101.0, //
    72.0, 92.0, 95.0, 98.0, 112.0, 100.0, 103.0, 99.0,
];

static DCT_K: Lazy<[[f64; 8 * 8]; 8 * 8]> = Lazy::<[[f64; 8 * 8]; 8 * 8]>::new(|| {
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

    pub fn dct(&self, dst: &mut Block) {
        for (uv, d) in dst.0.iter_mut().enumerate() {
            *d = self.0.iter().enumerate().map(|(xy, g)| g * DCT_K[uv][xy]).sum();
        }
    }

    pub fn quantization(&mut self) {
        for (d, q) in self.0.iter_mut().zip(QMATRIX) {
            *d = (*d / q).round();
        }
    }
}
