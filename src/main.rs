use core::f64;
use std::{
    cmp::{max, min},
    f64::consts::PI,
    io::Write,
    path::Path,
    time::Instant,
};

mod bitio;
mod blocks;
mod colors;
mod motion;
mod planes;
mod videocode;

use anyhow::Result;
use bitio::{BitReader, BitWriter};
use blocks::{Block, QMatrices};
use humansize::{format_size, BINARY};
use image::{GrayImage, ImageBuffer, ImageReader, Luma, Rgb, RgbImage};
use imageproc::drawing::BresenhamLineIter;
use motion::{BlockType, MotionMap};
use ndarray::{s, Array, Array2, ShapeBuilder};
use ndarray_stats::QuantileExt;
use once_cell::sync::Lazy;
use planes::Plane;
use videocode::{MacroBlock, VideoFrame};

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
*/
/*fn block_sad(a: &Plane, ax: u32, ay: u32, b: &Plane, bx: u32, by: u32) -> f64 {
    let mut accum = 0f64;
    for y in 0..8 {
        let astart = (ax + (ay + y) * a.width()) as usize;
        let bstart = (bx + (by + y) * b.width()) as usize;
        let aline = &a.data[astart..astart + 8];
        let bline = &b.data[bstart..bstart + 8];
        accum += aline
            .iter()
            .zip(bline.iter())
            .map(|(a, b)| (*a - *b).abs())
            .sum::<f64>();
    }
    return accum;
}

const ZMP_TRESHOLD: f64 = 512.0;*/

fn compress_plane(plane: &Plane, writer: &mut BitWriter, is_luma: bool, quality: f64) -> Result<()> {
    let mut block = Block::new();
    for by in 0..plane.height() / 8 {
        for bx in 0..plane.width() / 8 {
            plane.extract_block(bx * 8, by * 8, &mut block);
            block.normalize();
            block.encode2(writer, is_luma, quality)?;
        }
    }
    return Ok(());
}

fn unpack_plane(plane: &mut Plane, reader: &mut BitReader, is_luma: bool, quality: f64) -> Result<()> {
    let mut block = Block::new();
    for by in 0..plane.height() / 8 {
        for bx in 0..plane.width() / 8 {
            block.decode2(reader, is_luma, quality)?;
            block.denormalize();
            plane.apply_block(bx * 8, by * 8, &block);
        }
    }
    return Ok(());
}

const FILE_A: &str = "0690.png";
const FILE_B: &str = "0693.png";
const FILE_RES: &str = "0693_motion.png";
fn main() -> Result<()> {
    /*let mut test_block = Block([
        -76.0, -73.0, -67.0, -62.0, -58.0, -67.0, -64.0, -55.0, -65.0, -69.0, -73.0, -38.0, -19.0, -43.0, -59.0, -56.0,
        -66.0, -69.0, -60.0, -15.0, 16.0, -24.0, -62.0, -55.0, -65.0, -70.0, -57.0, -6.0, 26.0, -22.0, -58.0, -59.0,
        -61.0, -67.0, -60.0, -24.0, -2.0, -40.0, -60.0, -58.0, -49.0, -63.0, -68.0, -58.0, -51.0, -60.0, -70.0, -53.0,
        -43.0, -57.0, -64.0, -69.0, -73.0, -67.0, -63.0, -45.0, -41.0, -49.0, -59.0, -60.0, -63.0, -52.0, -50.0, -34.0,
    ]);

    let mut output = Vec::<u8>::new();
    let mut writer = BitWriter::new(&mut output);
    test_block.encode2(&mut writer, true, 0.9)?;
    writer.flush()?;
    let mut outslice = &output[..];
    let mut reader = BitReader::new(&mut outslice);
    test_block.decode2(&mut reader, true, 0.9)?;
    println!("{:?}", test_block);
    return Ok(());*/

    /*
    let mut calced = Block::new();
    let now = Instant::now();
    test_block.apply_dct(&mut calced);
    println!("old dct:{:?}", now.elapsed());
    let now = Instant::now();
    test_block.apply_dct2();
    println!("new dct:{:?}", now.elapsed());
    println!("Old\n{:?}", calced);
    println!("New\n{:?}", test_block);
    test_block.revert_dct2();
    println!("New\n{:?}", test_block);
    calced.revert_dct(&mut test_block);
    println!("Old\n{:?}", test_block);
    return Ok(());*/
    /*
    let (image_width, image_height) = ImageReader::open("data/vid/test13/0001.png")?.into_dimensions()?;
    let y_plane_width = (image_width as f64 / 16.0).ceil() as u32 * 16;
    let y_plane_height = (image_height as f64 / 16.0).ceil() as u32 * 16;
    let uv_plane_width = y_plane_width / 2;
    let uv_plane_height = y_plane_height / 2;

    let mut plane_ay = Plane::new(y_plane_width, y_plane_height);
    let mut plane_au = Plane::new(uv_plane_width, uv_plane_height);
    let mut plane_av = Plane::new(uv_plane_width, uv_plane_height);

    //let mut plane_by = Plane::new(y_plane_width, y_plane_height);
    //let mut plane_bu = Plane::new(uv_plane_width, uv_plane_height);
    //let mut plane_bv = Plane::new(uv_plane_width, uv_plane_height);
    const frame_count: i32 = 5851;
    let mut decode_time = 0f64;
    let mut total = 0u64;
    for i in 1..=frame_count {
        print!("{}/{} ", i, frame_count);
        Plane::image2planes(
            format!("data/vid/test13/{:04}.png", i),
            &mut plane_ay,
            &mut plane_au,
            &mut plane_av,
        )?;
        //Plane::image2planes("data/001.tif", &mut plane_by, &mut plane_bu, &mut plane_bv)?;
        let mut output = Vec::<u8>::new();
        let mut writer = BitWriter::new(&mut output);

        /*for (a, b) in plane_ay.data.iter_mut().zip(plane_by.data.iter_mut()) {
            *a -= *b;
        }
        for (a, b) in plane_au.data.iter_mut().zip(plane_bu.data.iter_mut()) {
            *a -= *b;
        }
        for (a, b) in plane_av.data.iter_mut().zip(plane_bv.data.iter_mut()) {
            *a -= *b;
        }*/

        //let mut result_y = ImageBuffer::new(y_plane_width, y_plane_height);
        //let mut result_u = ImageBuffer::new(uv_plane_width, uv_plane_height);
        //let mut result_v = ImageBuffer::new(uv_plane_width, uv_plane_height);
        //let mut result_full = ImageBuffer::new(image_width, image_height);

        //Plane::plane2luma(&plane_ay, &mut result_y);
        //Plane::plane2luma(&plane_au, &mut result_u);
        //Plane::plane2luma(&plane_av, &mut result_v);
        //Plane::planes2image(&plane_ay, &plane_au, &plane_av, &mut result_full);

        //result_y.save("data/result_y.png")?;
        //result_u.save("data/result_u.png")?;
        //result_v.save("data/result_v.png")?;
        //result_full.save("data/test6_c.png")?;

        let all_quality = 0.9;

        compress_plane(&plane_ay, &mut writer, true, all_quality)?;
        compress_plane(&plane_au, &mut writer, false, all_quality)?;
        compress_plane(&plane_av, &mut writer, false, all_quality)?;
        writer.flush()?;

        let len_compressed = output.len();
        total += len_compressed as u64;
        let len_uncompressed = image_width * image_height * 3;
        let compression_level = (len_compressed as f64 / len_uncompressed as f64 * 100.0).round() as u32;
        println!(
            "size {} of {} ({}%)",
            len_compressed, len_uncompressed, compression_level
        );

        let mut outslice = &output[..];
        let mut reader = BitReader::new(&mut outslice);

        let mut plane_ay_res = Plane::new(y_plane_width, y_plane_height);
        let mut plane_au_res = Plane::new(uv_plane_width, uv_plane_height);
        let mut plane_av_res = Plane::new(uv_plane_width, uv_plane_height);
        //let mut result_y_res = ImageBuffer::new(y_plane_width, y_plane_height);
        //let mut result_u_res = ImageBuffer::new(uv_plane_width, uv_plane_height);
        //let mut result_v_res = ImageBuffer::new(uv_plane_width, uv_plane_height);
        let mut result_full_res = ImageBuffer::new(image_width, image_height);

        //println!("Start decoding");
        let now = Instant::now();
        unpack_plane(&mut plane_ay_res, &mut reader, true, all_quality)?;
        unpack_plane(&mut plane_au_res, &mut reader, false, all_quality)?;
        unpack_plane(&mut plane_av_res, &mut reader, false, all_quality)?;

        //Plane::plane2luma(&plane_ay_res, &mut result_y_res);
        //Plane::plane2luma(&plane_au_res, &mut result_u_res);
        //Plane::plane2luma(&plane_av_res, &mut result_v_res);
        Plane::planes2image(&plane_ay_res, &plane_au_res, &plane_av_res, &mut result_full_res);
        let elapsed = now.elapsed();
        decode_time += elapsed.as_secs_f64();
        //println!("Decoded: {:?}", elapsed);
        //result_y_res.save("data/result_y_res.png")?;
        //result_u_res.save("data/result_u_res.png")?;
        //result_v_res.save("data/result_v_res.png")?;
        result_full_res.save(format!("data/vid/test13_result/{:04}.png", i))?;
    }
    let len_uncompressed = image_width as u64 * image_height as u64 * 3 * frame_count as u64;
    let compression_level = (total as f64 / len_uncompressed as f64 * 100.0).round() as u32;
    println!("Total size: {} of {} ({}%)", total, len_uncompressed, compression_level);
    println!(
        "Average decoding time: {}ms",
        decode_time / (frame_count as f64) * 1000.0
    );
    */
    /*let (image_width, image_height) = ImageReader::open("data/076.tif")?.into_dimensions()?;

    let mut frame_a = VideoFrame::new(image_width, image_height);
    let mut frame_b = VideoFrame::new(image_width, image_height);

    frame_a.load_from_image("data/076.tif")?;
    frame_b.load_from_image("data/079.tif")?;

    let mv_width = frame_a.width / 16;
    let mv_height = frame_a.height / 16;
    let mut block_map = vec![MacroblockType::New; (mv_width * mv_height) as usize];

    for my in 0..mv_height {
        for mx in 0..mv_width {
            let mv_index = (mx + my * mv_width) as usize;
            let dst_x = mx * 16;
            let dst_y = my * 16;

            let mut vect = (0i32, 0i32);
            let mut min_d = block_sad(&frame_b.y_plane, dst_x, dst_y, &frame_a.y_plane, dst_x, dst_y);
            if min_d > ZMP_TRESHOLD {
                for by in max(dst_y as i32 - 7, 0)..=min(dst_y as i32 + 7, frame_a.height as i32 - 1 - 16) {
                    for bx in max(dst_x as i32 - 7, 0)..=min(dst_x as i32 + 7, frame_a.width as i32 - 1 - 16) {
                        let new_d = block_sad(&frame_b.y_plane, dst_x, dst_y, &frame_a.y_plane, bx as u32, by as u32);
                        if new_d < min_d {
                            min_d = new_d;
                            vect = (bx - dst_x as i32, by - dst_y as i32);
                        }
                    }
                }
                if min_d > ZMP_TRESHOLD {
                    block_map[mv_index] = MacroblockType::New;
                } else {
                    block_map[mv_index] = MacroblockType::Motion(vect.0, vect.1);
                }
            } else {
                block_map[mv_index] = MacroblockType::Motion(0, 0);
            }
        }
    }

    let mut macroblock = MacroBlock::new();
    frame_b.u_plane.fill(0.0);
    frame_b.v_plane.fill(0.0);
    for my in 0..mv_height {
        for mx in 0..mv_width {
            let mv_index = (mx + my * mv_width) as usize;
            if let MacroblockType::Motion(vx, vy) = block_map[mv_index] {
                frame_a.extract_macroblock(
                    (mx as i32 * 16 + vx) as u32,
                    (my as i32 * 16 + vy) as u32,
                    &mut macroblock,
                );
                if vx == 0 && vy == 0 {
                    macroblock.0[4].0.fill(0.0);
                }
                frame_b.apply_macroblock(mx * 16, my * 16, &macroblock);
            }
        }
    }

    frame_b.save_to_image("data/076_1.png")?;*/
    let (image_width, image_height) = ImageReader::open(format!("data/{}", FILE_A))?.into_dimensions()?;

    let mut frame_a = VideoFrame::new(image_width, image_height);
    let mut frame_b = VideoFrame::new(image_width, image_height);

    frame_a.load_from_image(format!("data/{}", FILE_A))?;
    frame_b.load_from_image(format!("data/{}", FILE_B))?;

    let mv_width = frame_a.width / 16;
    let mv_height = frame_a.height / 16;

    let qmatrices = QMatrices::new(0.95);

    let mut motion = MotionMap::new(&frame_a);
    motion.calculate_ult(&frame_b, &frame_a, &qmatrices);

    /*let mut macroblock = MacroBlock::new();
    //frame_b.u_plane.fill(0.0);
    //frame_b.v_plane.fill(0.0);
    for my in 0..mv_height {
        for mx in 0..mv_width {
            let mv_index = (mx + my * mv_width) as usize;
            if let BlockType::Motion(vx, vy) = motion.vectors[mv_index] {
                frame_a.extract_macroblock(
                    (mx as i32 * 16 + vx) as u32,
                    (my as i32 * 16 + vy) as u32,
                    &mut macroblock,
                );
                if vx == 0 && vy == 0 {
                    // macroblock.0[4].0.fill(0.0);
                }
                frame_b.apply_macroblock(mx * 16, my * 16, &macroblock);
            }
        }
    }*/

    let mut output = Vec::<u8>::new();
    let mut writer = BitWriter::new(&mut output);

    let mut macroblock = MacroBlock::new();
    let mut macroblock2 = MacroBlock::new();

    for my in 0..mv_height {
        for mx in 0..mv_width {
            let dst_x = mx * 16;
            let dst_y = my * 16;
            let mv_index = (mx + my * mv_width) as usize;

            frame_b.extract_macroblock(dst_x, dst_y, &mut macroblock);

            if let BlockType::Motion(vx, vy) = motion.vectors[mv_index] {
                frame_a.extract_macroblock((dst_x as i32 + vx) as u32, (dst_y as i32 + vy) as u32, &mut macroblock2);
                macroblock.difference(&macroblock2);
                frame_b.apply_macroblock(dst_x, dst_y, &macroblock2);
            }

            macroblock.encode(&qmatrices);
            macroblock.write(&mut writer)?;

            macroblock.decode(&qmatrices);
            //frame_b.apply_macroblock(dst_x, dst_y, &macroblock);
        }
    }
    writer.flush()?;
    println!(
        "Frame compressed size: {} ({} bytes)",
        format_size(output.len(), BINARY),
        output.len()
    );

    frame_b.save_to_image(format!("data/{}", FILE_RES))?;
    /*

    let mut outslice = &output[..];
    let mut reader = BitReader::new(&mut outslice);

    for my in 0..mv_height {
        for mx in 0..mv_width {
            let dst_x = mx * 16;
            let dst_y = my * 16;

            macroblock.read(&mut reader)?;

            macroblock.decode(&qmatrices);
            frame.apply_macroblock(dst_x, dst_y, &macroblock);
        }
    }

    frame.save_to_image("data/test6_readed.png")?;*/
    Ok(())
}
