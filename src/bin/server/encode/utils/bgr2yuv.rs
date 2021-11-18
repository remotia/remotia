pub mod pixel {
    pub fn bgr_to_yuv(_b: u8, _g: u8, _r: u8) -> (u8, u8, u8) {
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

    pub fn bgr_to_yuv(bgr_pixels: &[u8], yuv_pixels: &mut [u8]) {
        let pixels_count = bgr_pixels.len() / 3;

        for i in 0..pixels_count {
            let (b, g, r) = (
                bgr_pixels[i * 3],
                bgr_pixels[i * 3 + 1],
                bgr_pixels[i * 3 + 2],
            );
            let (y, u, v) = pixel::bgr_to_yuv(b, g, r);

            let y_index = i;
            let u_index = pixels_count + i / 4;
            let v_index = pixels_count + pixels_count / 4 + i / 4;

            yuv_pixels[y_index] = y;
            yuv_pixels[u_index] += (u as f64 * 0.25) as u8;
            yuv_pixels[v_index] += (v as f64 * 0.25) as u8;
        }
    }

    #[allow(dead_code)]
    pub fn bgr_to_yuv_local_arrays(bgr_pixels: &[u8], yuv_pixels: &mut [u8]) {
        let pixels_count = bgr_pixels.len() / 3;

        let mut y_pixels: Vec<u8> = vec![0; pixels_count];
        let mut u_pixels: Vec<u8> = vec![0; pixels_count / 4];
        let mut v_pixels: Vec<u8> = vec![0; pixels_count / 4];

        for i in 0..pixels_count {
            let (b, g, r) = (
                bgr_pixels[i * 3],
                bgr_pixels[i * 3 + 1],
                bgr_pixels[i * 3 + 2],
            );
            let (y, u, v) = pixel::bgr_to_yuv(b, g, r);

            y_pixels[i] = y;
            u_pixels[i / 4] = (u as f64 * 0.25) as u8;
            v_pixels[i / 4] = (v as f64 * 0.25) as u8;
        }

        yuv_pixels[..pixels_count].copy_from_slice(&y_pixels);
        yuv_pixels[pixels_count..pixels_count+pixels_count/4].copy_from_slice(&u_pixels);
        yuv_pixels[pixels_count+pixels_count/4..].copy_from_slice(&v_pixels);
    }
}

#[cfg(test)]
mod tests {
    use log::debug;

    use crate::encode::utils::bgr2yuv::raster;

    #[test]
    fn bgr_to_yuv_simple_test() {
        // let input: Vec<u8> = vec![0, 64, 32, 0, 0, 0, 128, 255, 32, 128, 64, 32];
        let input: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut output: Vec<u8> = vec![0; input.len() / 2];

        raster::bgr_to_yuv(&input, &mut output);

        debug!("{:?}", output);
    }
}
