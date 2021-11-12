pub mod pixel {
    pub fn rgb_to_yuv(_r: u8, _g: u8, _b: u8) -> (u8, u8, u8) {
        let r = _r as f64;
        let g = _g as f64;
        let b = _b as f64;

        let y: u8 = (r * 0.29900 + g * 0.58700 + b * 0.11400) as u8;
        let u: u8 = ((r * -0.16874 + g * -0.33126 + b * 0.50000) as i16 + 128) as u8;
        let v: u8 = ((r * 0.50000 + g * -0.41869 + b * -0.08131) as i16 + 128) as u8;

        (y, u, v)
    }
}

pub mod raster {
    use super::pixel;

    pub fn rgb_to_yuv(rgb_pixels: &[u8], yuv_pixels: &mut [u8]) {
        let pixels_count = rgb_pixels.len() / 3;

        for i in 0..pixels_count {
            let (r, g, b) = (
                rgb_pixels[i * 3],
                rgb_pixels[i * 3 + 1],
                rgb_pixels[i * 3 + 2],
            );
            let (y, u, v) = pixel::rgb_to_yuv(r, g, b);

            let y_index = i;
            let u_index = pixels_count + i / 4;
            let v_index = pixels_count + pixels_count / 4 + i / 4;

            yuv_pixels[y_index] = y;
            yuv_pixels[u_index] += (u as f64 * 0.25) as u8;
            yuv_pixels[v_index] += (v as f64 * 0.25) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::encode::utils::rgb2yuv::raster;

    #[test]
    fn rgb_to_yuv_simple_test() {
        // let input: Vec<u8> = vec![0, 64, 32, 0, 0, 0, 128, 255, 32, 128, 64, 32];
        let input: Vec<u8> = vec![255, 255, 255, 0, 0, 0, 255, 255, 255, 0, 0, 0];
        let mut output: Vec<u8> = vec![0; input.len() / 2];

        raster::rgb_to_yuv(&input, &mut output);

        println!("{:?}", output);
    }
}
