pub mod pixel {
    pub fn yuv_to_rgb(_y: u8, _u: u8, _v: u8) -> (u8, u8, u8) {
        let y: f64 = _y as f64;
        let u: f64 = ((_u as i16) - 128) as f64;
        let v: f64 = ((_v as i16) - 128) as f64;

        let r = (y +                v *  1.40200) as u8;
        let g = (y + u * -0.34414 + v * -0.71414) as u8;
        let b = (y + u *  1.77200               ) as u8;

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

            rgb_pixels[i * 3] = r;
            rgb_pixels[i * 3 + 1] = g;
            rgb_pixels[i * 3 + 2] = b;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::decode::utils::yuv2rgb::raster;

    #[test]
    fn yuv_to_rgb_simple_test() {
        let input: Vec<u8> = vec![41, 0, 191, 79, 96, 116];
        let mut output: Vec<u8> = vec![0; input.len() * 2];

        raster::yuv_to_rgb(&input, &mut output);

        println!("{:?}", output);
    }
}
