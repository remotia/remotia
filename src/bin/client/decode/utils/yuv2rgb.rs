pub mod pixel {
    pub fn yuv_to_rgb(y: u8, u: u8, v: u8) -> (u8, u8, u8) {
        let y = (y - 16) as f64;
        let u = (u - 128) as f64;
        let v = (v - 128) as f64;

        let r = (1.164 * y             + 1.596 * v) as u8;
        let g = (1.164 * y - 0.392 * u - 0.813 * v) as u8;
        let b = (1.164 * y + 2.017 * u            ) as u8;

        (r, g, b)
    }
}

pub mod raster {
    use super::pixel;

    pub fn yuv_to_rgb(yuv_pixels: &[u8], rgb_pixels: &mut [u8]) {
        let pixels_count = rgb_pixels.len() / 3;

        for i in 0..pixels_count {
            let (y, u, v) = (
                yuv_pixels[i],
                yuv_pixels[pixels_count + i / 4],
                yuv_pixels[pixels_count + pixels_count / 4 + i / 4],
            );

            let (r, g, b) = pixel::yuv_to_rgb(y, u, v);

            rgb_pixels[i] = r;
            rgb_pixels[i + 1] = g;
            rgb_pixels[i + 2] = b;
        }
    }
}
