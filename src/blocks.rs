use std::{f64::consts::PI, fmt};

use once_cell::sync::Lazy;

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
        for (d, q) in self.0.iter_mut().zip(QMATRIX_LUMA) {
            *d = (*d / q).round();
        }
    }

    pub fn unwrap(&self, dst: &mut Block) {
        for (i, d) in dst.0.iter_mut().enumerate() {
            *d = self.0[UNWRAP_PATTERN[i]];
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

    pub fn encode(&self) {
        println!("{:?}", self.0);
        let dc = self.0[0] as i16;
        print!("({}) {}; ", Block::int_width(dc), dc);
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
                    print!("(15, 0); ");
                    zeroes = 0;
                }
            } else {
                let binary = format!("{:16b}", if item < 0 { item - 1 } else { item });
                let item_width = Block::int_width(item);
                print!("([{}, {}], {}); ", zeroes, item_width, &binary[16 - item_width..]);
                zeroes = 0;
            }
        }
        if tail > 0 {
            print!("(0, 0); ");
        }
    }
}
