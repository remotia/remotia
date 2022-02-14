pub mod pixel {
    pub fn yuv_to_bgr(_y: u8, _u: u8, _v: u8) -> (u8, u8, u8) {
        let y: f64 = _y as f64;
        let u: f64 = ((_u as i16) - 128) as f64;
        let v: f64 = ((_v as i16) - 128) as f64;

        let b = (y + u *  1.77200               ) as u8;
        let g = (y + u * -0.34414 + v * -0.71414) as u8;
        let r = (y +                v *  1.40200) as u8;

        (b, g, r)
    }
}

pub mod raster {
    use super::pixel;

    pub fn yuv_to_bgr(yuv_pixels: &[u8], bgr_pixels: &mut [u8]) {
        let pixels_count = bgr_pixels.len() / 3;

        for i in 0..pixels_count {
            let (y, u, v) = (
                yuv_pixels[i],
                yuv_pixels[pixels_count + i / 4],
                yuv_pixels[pixels_count + pixels_count / 4 + i / 4],
            );

            let (b, g, r) = pixel::yuv_to_bgr(y, u, v);

            bgr_pixels[i * 3] = b;
            bgr_pixels[i * 3 + 1] = g;
            bgr_pixels[i * 3 + 2] = r;
        }
    }

    pub fn yuv_to_bgra(yuv_pixels: &[u8], bgra_pixels: &mut [u8]) {
        let pixels_count = bgra_pixels.len() / 4;

        for i in 0..pixels_count {
            let (y, u, v) = (
                yuv_pixels[i],
                yuv_pixels[pixels_count + i / 4],
                yuv_pixels[pixels_count + pixels_count / 4 + i / 4],
            );

            let (b, g, r) = pixel::yuv_to_bgr(y, u, v);

            bgra_pixels[i * 4] = b;
            bgra_pixels[i * 4 + 1] = g;
            bgra_pixels[i * 4 + 2] = r;
            bgra_pixels[i * 4 + 3] = 255;
        }
    }

    pub fn yuv_to_bgra_separate(y_pixels: &[u8], u_pixels: &[u8], v_pixels: &[u8], bgra_pixels: &mut [u8]) {
        let pixels_count = bgra_pixels.len() / 4;

        for i in 0..pixels_count {
            let (y, u, v) = (
                y_pixels[i],
                u_pixels[i / 4],
                v_pixels[i / 4],
            );

            let (b, g, r) = pixel::yuv_to_bgr(y, u, v);

            bgra_pixels[i * 4] = b;
            bgra_pixels[i * 4 + 1] = g;
            bgra_pixels[i * 4 + 2] = r;
            bgra_pixels[i * 4 + 3] = 255;
        }
    }
}

#[cfg(test)]
mod tests {
    use log::debug;

    use crate::decoders::utils::yuv2bgr::raster;

    #[test]
    fn yuv_to_bgr_simple_test() {
        let input: Vec<u8> = vec![41, 0, 191, 79, 96, 116];
        let mut output: Vec<u8> = vec![0; input.len() * 2];

        raster::yuv_to_bgr(&input, &mut output);

        debug!("{:?}", output);
    }
}
