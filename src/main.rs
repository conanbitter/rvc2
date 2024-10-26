use core::f64;
use std::{
    cmp::{max, min},
    f64::consts::PI,
    path::Path,
};

mod bitio;
mod blocks;
mod planes;

use anyhow::Result;
use bitio::{BitReader, BitWriter};
use blocks::Block;
use image::{GrayImage, ImageBuffer, ImageReader, Luma, Rgb, RgbImage};
use imageproc::drawing::BresenhamLineIter;
use ndarray::{s, Array, Array2, ShapeBuilder};
use ndarray_stats::QuantileExt;
use once_cell::sync::Lazy;
use planes::Plane;

/*
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

fn load_planes<P: AsRef<Path>>(
    filename: P,
    yp: &mut Array2<f64>,
    up: &mut Array2<f64>,
    vp: &mut Array2<f64>,
) -> Result<()> {
    let img = ImageReader::open(filename)?.decode()?.to_rgb8();

    let image_width = img.width() as usize;
    let image_height = img.height() as usize;
    let y_plane_width = (image_width as f64 / 16.0).ceil() as usize * 16;
    let y_plane_height = (image_height as f64 / 16.0).ceil() as usize * 16;

    for py in 0..y_plane_height {
        for px in 0..y_plane_width {
            let ix = px.min(image_width - 1) as u32;
            let iy = py.min(image_height - 1) as u32;
            let Rgb([r, g, b]) = *img.get_pixel(ix, iy);

            let (y, u, v) = rgb2yuv(r, g, b);
            yp[(px, py)] = y;
            up[(px / 2, py / 2)] += u;
            vp[(px / 2, py / 2)] += v;
        }
    }

    *up /= 4.0;
    *vp /= 4.0;

    return Ok(());
}

fn block_sad(a: &Array2<f64>, ax: usize, ay: usize, b: &Array2<f64>, bx: usize, by: usize) -> f64 {
    let block1 = a.slice(s![ax..ax + 8, ay..ay + 8]);
    let block2 = b.slice(s![bx..bx + 8, by..by + 8]);
    return block1.iter().zip(block2.iter()).map(|(a, b)| (a - b).abs()).sum();
}

const ZMP_TRESHOLD: f64 = 128.0; //512.0;
*/
fn main() -> Result<()> {
    let mut output = Vec::<u8>::new();
    let mut writer = BitWriter::new(&mut output);
    let mut output2 = Vec::<u8>::new();
    let mut writer2 = BitWriter::new(&mut output2);

    let mut test_block = Block([
        -76.0, -73.0, -67.0, -62.0, -58.0, -67.0, -64.0, -55.0, -65.0, -69.0, -73.0, -38.0, -19.0, -43.0, -59.0, -56.0,
        -66.0, -69.0, -60.0, -15.0, 16.0, -24.0, -62.0, -55.0, -65.0, -70.0, -57.0, -6.0, 26.0, -22.0, -58.0, -59.0,
        -61.0, -67.0, -60.0, -24.0, -2.0, -40.0, -60.0, -58.0, -49.0, -63.0, -68.0, -58.0, -51.0, -60.0, -70.0, -53.0,
        -43.0, -57.0, -64.0, -69.0, -73.0, -67.0, -63.0, -45.0, -41.0, -49.0, -59.0, -60.0, -63.0, -52.0, -50.0, -34.0,
    ]);

    test_block.encode2(&mut writer2)?;
    writer2.flush()?;

    let mut calced = Block::new();

    test_block.apply_dct(&mut calced);
    calced.quantization();
    calced.unwrap(&mut test_block);
    println!("{:?}", test_block);

    test_block.encode(&mut writer)?;
    writer.flush()?;
    for i in &output {
        print!("{:08b} ", i);
    }
    println!("");
    for i in &output2 {
        print!("{:08b} ", i);
    }
    println!("");

    let mut outslice = &output[..];
    let mut reader = BitReader::new(&mut outslice);

    let mut test_block2 = Block::new();
    test_block2.decode(&mut reader)?;
    test_block2.wrap(&mut test_block);
    test_block.dequantization();
    //test_block.unwrap(&mut test_block2);
    test_block.revert_dct(&mut test_block2);
    println!("{:?}", test_block2);

    let mut outslice2 = &output2[..];
    let mut reader2 = BitReader::new(&mut outslice2);
    let mut test_block3 = Block::new();
    test_block3.decode2(&mut reader2)?;
    test_block3.substract(&test_block2);
    println!("{:?}", test_block3);

    return Ok(());

    let (image_width, image_height) = ImageReader::open("data/056.tif")?.into_dimensions()?;
    let y_plane_width = (image_width as f64 / 16.0).ceil() as u32 * 16;
    let y_plane_height = (image_height as f64 / 16.0).ceil() as u32 * 16;
    let uv_plane_width = y_plane_width / 2;
    let uv_plane_height = y_plane_height / 2;

    let mut plane_ay = Plane::new(y_plane_width, y_plane_height);
    let mut plane_au = Plane::new(uv_plane_width, uv_plane_height);
    let mut plane_av = Plane::new(uv_plane_width, uv_plane_height);

    let mut plane_by = Plane::new(y_plane_width, y_plane_height);
    let mut plane_bu = Plane::new(uv_plane_width, uv_plane_height);
    let mut plane_bv = Plane::new(uv_plane_width, uv_plane_height);

    Plane::image2planes("data/056.tif", &mut plane_ay, &mut plane_au, &mut plane_av)?;
    Plane::image2planes("data/059.tif", &mut plane_by, &mut plane_bu, &mut plane_bv)?;

    let mut result_y = ImageBuffer::new(y_plane_width, y_plane_height);
    let mut result_u = ImageBuffer::new(uv_plane_width, uv_plane_height);
    let mut result_v = ImageBuffer::new(uv_plane_width, uv_plane_height);
    let mut result_full = ImageBuffer::new(image_width, image_height);

    Plane::plane2luma(&plane_by, &mut result_y);
    Plane::plane2luma(&plane_bu, &mut result_u);
    Plane::plane2luma(&plane_bv, &mut result_v);
    Plane::planes2image(&plane_by, &plane_bu, &plane_bv, &mut result_full);

    result_y.save("data/result_y.png")?;
    result_u.save("data/result_u.png")?;
    result_v.save("data/result_v.png")?;
    result_full.save("data/056_1.png")?;

    /*
        let img = ImageReader::open("data/056.tif")?.decode()?.to_rgb8();

        let image_width = img.width() as usize;
        let image_height = img.height() as usize;
        let y_plane_width = (image_width as f64 / 16.0).ceil() as usize * 16;
        let y_plane_height = (image_height as f64 / 16.0).ceil() as usize * 16;
        let uv_plane_width = y_plane_width / 2;
        let uv_plane_height = y_plane_height / 2;

        println!(
            "    image: {}x{}\n  Y plane: {}x{}\nUV planes: {}x{}",
            image_width, image_height, y_plane_width, y_plane_height, uv_plane_width, uv_plane_height
        );

        let y_dims = (y_plane_width, y_plane_height).f();
        let uv_dims = (uv_plane_width, uv_plane_height).f();

        let mut y_plane = Array2::<f64>::zeros(y_dims);
        let mut u_plane = Array2::<f64>::zeros(uv_dims);
        let mut v_plane = Array2::<f64>::zeros(uv_dims);

        let mut y_plane2 = Array2::<f64>::zeros(y_dims);
        let mut u_plane2 = Array2::<f64>::zeros(uv_dims);
        let mut v_plane2 = Array2::<f64>::zeros(uv_dims);

        load_planes("data/056.tif", &mut y_plane, &mut u_plane, &mut v_plane)?;
        load_planes("data/059.tif", &mut y_plane2, &mut u_plane2, &mut v_plane2)?;

        let mut block_diff = Array2::<(i32, i32)>::from_elem((y_plane_width / 8, y_plane_height / 8).f(), (0, 0));
        for ((x, y), d) in block_diff.indexed_iter_mut() {
            let dst_x = x * 8;
            let dst_y = y * 8;
            let mut vect = (0i32, 0i32);
            let mut min_d = block_sad(&y_plane2, dst_x, dst_y, &y_plane, dst_x, dst_y);

            if min_d > ZMP_TRESHOLD {
                for by in max(dst_y as i32 - 8, 0)..=min(dst_y as i32 + 8, y_plane_height as i32 - 1 - 8) {
                    for bx in max(dst_x as i32 - 8, 0)..=min(dst_x as i32 + 8, y_plane_width as i32 - 1 - 8) {
                        let new_d = block_sad(&y_plane2, dst_x, dst_y, &y_plane, bx as usize, by as usize);
                        if new_d < min_d {
                            min_d = new_d;
                            vect = (bx - dst_x as i32, by - dst_y as i32);
                        }
                    }
                }
                if min_d > ZMP_TRESHOLD {
                    *d = (100, 100);
                } else {
                    *d = vect
                };
            }
            *d = vect;
            println!("x: {} y:{}", x, y);
        }

        //println!("max diff: {:?}", block_diff);

        /*let mut result_diff = ImageBuffer::new(y_plane_width as u32, y_plane_height as u32);
        for py in 0..y_plane_height {
            for px in 0..y_plane_width {
                let (vx, vy) = block_diff[(px / 8, py / 8)];
                let r = ((vx + 8) * 15) as u8;
                let g = ((vy + 8) * 15) as u8;
                let c = Rgb([r, g, 0]);

                result_diff.put_pixel(px as u32, py as u32, c);
            }
        }

        result_diff.save("data/result_diff.png")?;*/

        let mut result_y = ImageBuffer::new(y_plane_width as u32, y_plane_height as u32);
        let mut result_u = ImageBuffer::new(uv_plane_width as u32, uv_plane_height as u32);
        let mut result_v = ImageBuffer::new(uv_plane_width as u32, uv_plane_height as u32);
        let mut result_full = ImageBuffer::new(image_width as u32, image_height as u32);

        for py in 0..y_plane_height {
            for px in 0..y_plane_width {
                let y = y_plane2[(px, py)];
                let u = u_plane2[(px / 2, py / 2)];
                let v = v_plane2[(px / 2, py / 2)];
                let (r, g, b) = yuv2rgb(y, u, v);

                result_y.put_pixel(px as u32, py as u32, Luma([y as u8]));
                result_u.put_pixel(px as u32 / 2, py as u32 / 2, Luma([u as u8]));
                result_v.put_pixel(px as u32 / 2, py as u32 / 2, Luma([v as u8]));
                if px < image_width && py < image_height {
                    result_full.put_pixel(px as u32, py as u32, Rgb([r, g, b]));
                }
            }
        }

        for py in 0..y_plane_height / 8 {
            for px in 0..y_plane_width / 8 {
                let (vx, vy) = block_diff[(px, py)];
                if vx == 0 && vy == 0 {
                    continue;
                }
                if vx >= 100 {
                    let lx = (px * 8 + 4) as u32;
                    let ly = (py * 8 + 4) as u32;
                    if lx < image_width as u32 && ly < image_height as u32 {
                        result_full.put_pixel(lx, ly, Rgb([255, 0, 0]));
                    }
                } else {
                    let start = ((px * 8 + 4) as f32, (py * 8 + 4) as f32);
                    let end = ((px as i32 * 8 + 4 + vx) as f32, (py as i32 * 8 + 4 + vy) as f32);

                    let liner = BresenhamLineIter::new(start, end);
                    for (lx, ly) in liner {
                        if lx >= 0 && lx < image_width as i32 && ly >= 0 && ly < image_height as i32 {
                            result_full.put_pixel(lx as u32, ly as u32, Rgb([0, 255, 0]));
                        };
                    }
                }
            }
        }

        result_y.save("data/result_y.png")?;
        result_u.save("data/result_u.png")?;
        result_v.save("data/result_v.png")?;
        result_full.save("data/56_1.png")?;
    */
    Ok(())
}
