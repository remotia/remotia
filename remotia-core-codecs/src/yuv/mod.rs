pub mod rgba_to_yuv;
pub mod yuv_to_rgba;

pub fn bgr_to_yuv_f32(b: u8, g: u8, r: u8) -> (f32, f32, f32) {
    let r = r as f32;
    let g = g as f32;
    let b = b as f32;

    let y = r * 0.29900 + g * 0.58700 + b * 0.11400;
    let u = (r * -0.16874 + g * -0.33126 + b * 0.50000) + 128.0;
    let v = (r * 0.50000 + g * -0.41869 + b * -0.08131) + 128.0;

    (y, u, v)
}

#[inline]
pub fn yuv_to_rgb(y: u8, u: u8, v: u8) -> (u8, u8, u8) {
    let y: f64 = y as f64;
    let u: f64 = ((u as i16) - 128) as f64;
    let v: f64 = ((v as i16) - 128) as f64;

    let r = (y + v * 1.40200) as u8;
    let g = (y + u * -0.34414 + v * -0.71414) as u8;
    let b = (y + u * 1.77200) as u8;

    (r, g, b)
}

