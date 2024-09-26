use anyhow::Result;
use image::{ImageBuffer, ImageReader, Luma, Rgb};

type Block = [f64; 8 * 8];

fn get_y(r: f64, g: f64, b: f64) -> f64 {
    return 0.299 * r + 0.587 * g + 0.114 * b;
}

fn get_u(r: f64, g: f64, b: f64) -> f64 {
    return 128.0 - 0.168736 * r - 0.331264 * g + 0.5 * b;
}

fn get_v(r: f64, g: f64, b: f64) -> f64 {
    return 128.0 + 0.5 * r - 0.418688 * g - 0.081312 * b;
}

fn get_r(y: f64, v: f64) -> u8 {
    return (y + 1.402 * (v - 128.0)) as u8;
}

fn get_g(y: f64, u: f64, v: f64) -> u8 {
    return (y - 0.344136 * (u - 128.0) - 0.714136 * (v - 128.0)) as u8;
}

fn get_b(y: f64, u: f64) -> u8 {
    return (y + 1.772 * (u - 128.0)) as u8;
}

fn main() -> Result<()> {
    let img = ImageReader::open("data/test5.png")?.decode()?.to_rgb8();

    let image_width = img.width();
    let image_height = img.height();

    let luma_block_width = (image_width as f64 / 8.0).ceil() as u32;
    let luma_block_height = (image_height as f64 / 8.0).ceil() as u32;
    let color_block_width = (image_width as f64 / 16.0).ceil() as u32;
    let color_block_height = (image_height as f64 / 16.0).ceil() as u32;

    let mut y_plane: Vec<Block> = Vec::with_capacity((luma_block_width * luma_block_height) as usize);
    let mut u_plane: Vec<Block> = Vec::with_capacity((color_block_width * color_block_height) as usize);
    let mut v_plane: Vec<Block> = Vec::with_capacity((color_block_width * color_block_height) as usize);

    for y in 0..luma_block_height {
        for x in 0..luma_block_width {
            let mut new_block: Block = [0.0; 8 * 8];
            for by in 0..8 {
                for bx in 0..8 {
                    let block_index = (bx + by * 8) as usize;
                    let image_x = (x * 8 + bx).clamp(0, image_width - 1);
                    let image_y = (y * 8 + by).clamp(0, image_height - 1);
                    let Rgb([r, g, b]) = *img.get_pixel(image_x, image_y);
                    new_block[block_index] = get_y(r as f64, g as f64, b as f64);
                }
            }
            y_plane.push(new_block);
        }
    }

    for y in 0..color_block_height {
        for x in 0..color_block_width {
            let mut new_u_block: Block = [0.0; 8 * 8];
            let mut new_v_block: Block = [0.0; 8 * 8];
            for by in 0..8 {
                for bx in 0..8 {
                    let mut pr = 0.0;
                    let mut pg = 0.0;
                    let mut pb = 0.0;
                    let block_index = (bx + by * 8) as usize;
                    for sy in 0..2 {
                        for sx in 0..2 {
                            let image_x = (x * 16 + bx * 2 + sx).clamp(0, image_width - 1);
                            let image_y = (y * 16 + by * 2 + sy).clamp(0, image_height - 1);
                            let Rgb([r, g, b]) = *img.get_pixel(image_x, image_y);
                            pr += r as f64;
                            pg += g as f64;
                            pb += b as f64;
                        }
                    }
                    pr /= 4.0;
                    pg /= 4.0;
                    pb /= 4.0;
                    new_u_block[block_index] = get_u(pr, pg, pb);
                    new_v_block[block_index] = get_v(pr, pg, pb);
                }
            }
            u_plane.push(new_u_block);
            v_plane.push(new_v_block);
        }
    }

    let mut result_y = ImageBuffer::new(luma_block_width * 8, luma_block_height * 8);
    for (x, y, pixel) in result_y.enumerate_pixels_mut() {
        let plane_index = (x / 8 + y / 8 * luma_block_width) as usize;
        let block_index = (x % 8 + (y % 8) * 8) as usize;
        *pixel = Luma([y_plane[plane_index][block_index] as u8]);
    }
    result_y.save("data/result_y.png")?;

    let mut result_u = ImageBuffer::new(color_block_width * 8, color_block_height * 8);
    for (x, y, pixel) in result_u.enumerate_pixels_mut() {
        let plane_index = (x / 8 + y / 8 * color_block_width) as usize;
        let block_index = (x % 8 + (y % 8) * 8) as usize;
        *pixel = Luma([u_plane[plane_index][block_index] as u8]);
    }
    result_u.save("data/result_u.png")?;

    let mut result_v = ImageBuffer::new(color_block_width * 8, color_block_height * 8);
    for (x, y, pixel) in result_v.enumerate_pixels_mut() {
        let plane_index = (x / 8 + y / 8 * color_block_width) as usize;
        let block_index = (x % 8 + (y % 8) * 8) as usize;
        *pixel = Luma([v_plane[plane_index][block_index] as u8]);
    }
    result_v.save("data/result_v.png")?;

    let mut result_full = ImageBuffer::new(image_width, image_height);
    for (x, y, pixel) in result_full.enumerate_pixels_mut() {
        let luma_plane_index = (x / 8 + y / 8 * luma_block_width) as usize;
        let luma_block_index = (x % 8 + (y % 8) * 8) as usize;
        let color_plane_index = (x / 16 + y / 16 * color_block_width) as usize;
        let color_block_index = ((x % 16) / 2 + (y % 16) / 2 * 8) as usize;
        let y = y_plane[luma_plane_index][luma_block_index];
        let u = u_plane[color_plane_index][color_block_index];
        let v = v_plane[color_plane_index][color_block_index];
        *pixel = Rgb([get_r(y, v), get_g(y, u, v), get_b(y, u)]);
    }
    result_full.save("data/result.png")?;

    Ok(())
}
