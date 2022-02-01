#[allow(dead_code)]
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

#[allow(dead_code)]
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

    pub fn bgr_to_yuv_separate(
        bgr_pixels: &[u8],
        y_pixels: &mut [u8],
        u_pixels: &mut [u8],
        v_pixels: &mut [u8],
    ) {
        let pixels_count = bgr_pixels.len() / 3;

        for i in 0..pixels_count {
            let (b, g, r) = (
                bgr_pixels[i * 3],
                bgr_pixels[i * 3 + 1],
                bgr_pixels[i * 3 + 2],
            );
            let (y, u, v) = pixel::bgr_to_yuv(b, g, r);

            y_pixels[i] = y;
            u_pixels[i / 4] += (u as f64 * 0.25) as u8;
            v_pixels[i / 4] += (v as f64 * 0.25) as u8;
        }
    }

    pub fn bgra_to_yuv_separate(
        bgra_pixels: &[u8],
        y_pixels: &mut [u8],
        u_pixels: &mut [u8],
        v_pixels: &mut [u8],
    ) {
        let pixels_count = bgra_pixels.len() / 4;

        for i in 0..pixels_count {
            let (b, g, r) = (
                bgra_pixels[i * 4],
                bgra_pixels[i * 4 + 1],
                bgra_pixels[i * 4 + 2],
            );
            let (y, u, v) = pixel::bgr_to_yuv(b, g, r);

            y_pixels[i] = y;
            u_pixels[i / 4] += (u as f64 * 0.25) as u8;
            v_pixels[i / 4] += (v as f64 * 0.25) as u8;
        }
    }
}

#[cfg(test)]
#[allow(soft_unstable)]
mod tests {
    extern crate test;

    use log::debug;
    use log::info;
    use rand::prelude::StdRng;
    use rand::Rng;
    use rand::SeedableRng;
    use test::bench::BenchSamples;
    use test::Bencher;

    use crate::server::utils::bgr2yuv::raster;

    #[test]
    fn bgr_to_yuv_simple_test() {
        let input: Vec<u8> = vec![0, 64, 32, 0, 0, 0, 128, 255, 32, 128, 64, 32];
        // let input: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut output: Vec<u8> = vec![0; input.len() / 2];

        raster::bgr_to_yuv(&input, &mut output);

        println!("{:?}", output);
    }

    #[test]
    fn bgr_to_yuv_separate_simple_test() {
        let input: Vec<u8> = vec![0, 64, 32, 0, 0, 0, 128, 255, 32, 128, 64, 32];
        // let input: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut y_output: Vec<u8> = vec![0; 4];
        let mut u_output: Vec<u8> = vec![0; 1];
        let mut v_output: Vec<u8> = vec![0; 1];

        raster::bgr_to_yuv_separate(&input, &mut y_output, &mut u_output, &mut v_output);

        println!("{:?} {:?} {:?}", y_output, u_output, v_output);
    }

    fn generate_hd_frames_set(frames_count: i32) -> Vec<Vec<u8>> {
        let width = 1280;
        let height = 720;
        let pixels_count = width * height;
        let values_count = pixels_count * 3;

        info!("Generating {} {}x{} frames...", frames_count, width, height);

        let mut frames: Vec<Vec<u8>> = Vec::new();

        let mut rng = StdRng::seed_from_u64(42);

        for _ in 0..frames_count {
            let mut frame = vec![0; values_count];

            for p in 0..values_count {
                frame[p] = rng.gen();
            }

            frames.push(frame);
        }

        frames
    }

    fn bench_conversion_function(b: &mut Bencher, conversion_function: fn(&[u8], &mut [u8])) {
        let frames = generate_hd_frames_set(4);
        let mut output_buffer = vec![0; frames[0].len() / 2];

        info!("Running conversions...");
        b.iter(|| {
            frames.clone().into_iter().for_each(|frame| {
                conversion_function(frame.as_slice(), &mut output_buffer);
            });
        });
    }

    fn bench_conversion_function_with_separate_outputs(
        b: &mut Bencher,
        conversion_function: fn(&[u8], &mut [u8], &mut [u8], &mut [u8]),
    ) {
        let frames = generate_hd_frames_set(4);
        let pixels_count = frames[0].len() / 3;

        let mut y_output_buffer = vec![0; pixels_count];
        let mut u_output_buffer = vec![0; pixels_count / 4];
        let mut v_output_buffer = vec![0; pixels_count / 4];

        info!("Running conversions...");
        b.iter(|| {
            frames.clone().into_iter().for_each(|frame| {
                conversion_function(
                    frame.as_slice(),
                    &mut y_output_buffer,
                    &mut u_output_buffer,
                    &mut v_output_buffer,
                );
            });
        });
    }

    #[bench]
    fn bench_rgb_to_yuv(b: &mut Bencher) {
        env_logger::try_init().ok();
        info!("Benchmarking rgb_to_yuv...");
        bench_conversion_function(b, raster::bgr_to_yuv);
    }

    #[bench]
    fn bench_rgb_to_yuv_separate(b: &mut Bencher) {
        env_logger::try_init().ok();
        info!("Benchmarking rgb_to_yuv_separate...");
        bench_conversion_function_with_separate_outputs(b, raster::bgr_to_yuv_separate);
    }
}
