pub fn rgb2yuv(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let r = r as f64;
    let g = g as f64;
    let b = b as f64;

    let y = 0.299 * r + 0.587 * g + 0.114 * b;
    let u = 0.5 * (b - y) / (1.0 - 0.114) + 128.0;
    let v = 0.5 * (r - y) / (1.0 - 0.299) + 128.0;

    return (y, u, v);
}

pub fn yuv2rgb(y: f64, u: f64, v: f64) -> (u8, u8, u8) {
    let r = y + 1.402 * (v - 128.0);
    let g = y - (0.114 * 1.772 * (u - 128.0) + 0.299 * 1.402 * (v - 128.0)) / 0.587;
    let b = y + 1.772 * (u - 128.0);
    return (
        r.clamp(0.0, 255.0) as u8,
        g.clamp(0.0, 255.0) as u8,
        b.clamp(0.0, 255.0) as u8,
    );
}
