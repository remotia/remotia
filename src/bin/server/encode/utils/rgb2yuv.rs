pub mod pixel {
    pub fn rgb_to_yuv(r: u8, g: u8, b: u8) -> (u8, u8, u8) {
        let r = r as f64;
        let g = g as f64;
        let b = b as f64;

        let y = ( 0.257 * r + 0.504 * g + 0.098 * b) as u8 +  16;
        let u = (-0.148 * r - 0.291 * g + 0.439 * b) as u8 + 128;
        let v = ( 0.439 * r - 0.368 * g - 0.071 * b) as u8 + 128;

        (y, u, v)
    }
}

pub mod raster {
    use super::pixel;

    pub fn rgb_to_yuv(rgb_pixels: &[u8], yuv_pixels: &mut [u8]) {
        let pixels_count = rgb_pixels.len() / 3;

        for i in 0..pixels_count {
            let (r, g, b) = (rgb_pixels[i], rgb_pixels[i + 1], rgb_pixels[i + 2]);
            let (y, u, v) = pixel::rgb_to_yuv(r, g, b);

            yuv_pixels[i] = y;
            yuv_pixels[pixels_count + i / 4] = u;
            yuv_pixels[pixels_count + pixels_count / 4 + i / 4] = v;
        }
    }
}

