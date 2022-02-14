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

    pub fn bgr_to_yuv_f32(_b: u8, _g: u8, _r: u8) -> (f32, f32, f32) {
        let r = _r as f32;
        let g = _g as f32;
        let b = _b as f32;

        let y = r * 0.29900 + g * 0.58700 + b * 0.11400;
        let u = (r * -0.16874 + g * -0.33126 + b * 0.50000) + 128.0;
        let v = (r * 0.50000 + g * -0.41869 + b * -0.08131) + 128.0;

        (y, u, v)
    }
}

#[allow(dead_code)]
pub mod raster {
    use rayon::{
        iter::{
            IndexedParallelIterator, IntoParallelIterator, IntoParallelRefMutIterator,
            ParallelIterator,
        },
        slice::ParallelSlice,
    };

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

        let yuv_pixels = (0..pixels_count)
            .into_par_iter()
            .map(|i| {
                let (b, g, r) = (
                    bgra_pixels[i * 4],
                    bgra_pixels[i * 4 + 1],
                    bgra_pixels[i * 4 + 2],
                );

                (i, pixel::bgr_to_yuv_f32(b, g, r))
            })
            .collect::<Vec<(usize, (f32, f32, f32))>>();

        /*y_pixels.par_iter_mut().zip(yuv_pixels.clone()).for_each(|(value, (y, _, _))| {
            *value = y as u8;
        });

        u_pixels.par_iter_mut().zip(yuv_pixels.clone().chunks(4)).for_each(|(value, chunk)| {
            *value = chunk.iter().map(|(_, u, _)| (u * 0.25) as u8).sum();
        });

        v_pixels.par_iter_mut().zip(yuv_pixels.clone().chunks(4)).for_each(|(value, chunk)| {
            *value = chunk.iter().map(|(_, _, v)| (v * 0.25) as u8).sum();
        });*/

        yuv_pixels.into_iter().for_each(|(i, (y, u, v))| {
            y_pixels[i] = y as u8;
            u_pixels[i / 4] += (u * 0.25) as u8;
            v_pixels[i / 4] += (v * 0.25) as u8;
        });
    }
}

pub mod raster_simd {
    use std::{
        simd::{f32x16, f32x4, simd_swizzle, u8x16, u8x4},
        time::Instant,
    };

    use log::debug;

    fn clamp(val: f32) -> u8 {
        if val < 0.0 {
            0
        } else if val > 255.0 {
            255
        } else {
            val.round() as u8
        }
    }

    pub fn bgra_to_yuv_separate(
        bgra_pixels: &[u8],
        y_pixels: &mut [u8],
        u_pixels: &mut [u8],
        v_pixels: &mut [u8],
    ) {
        // static Y_COEFF: f32x4 = f32x4::from_array([0.114000, 0.587000, 0.299000, 0.0]);
        // static U_COEFF: f32x4 = f32x4::from_array([0.500000, -0.331264, -0.168736, 128.0]);
        // static V_COEFF: f32x4 = f32x4::from_array([-0.081312, -0.418688, 0.500000, 128.0]);

        static Y_COEFF: f32x16 = f32x16::from_array([
            0.114000, 0.587000, 0.299000, 0.0, 0.114000, 0.587000, 0.299000, 0.0, 0.114000,
            0.587000, 0.299000, 0.0, 0.114000, 0.587000, 0.299000, 0.0,
        ]);
        static U_COEFF: f32x16 = f32x16::from_array([
            0.500000, -0.331264, -0.168736, 128.0, 0.500000, -0.331264, -0.168736, 128.0, 0.500000,
            -0.331264, -0.168736, 128.0, 0.500000, -0.331264, -0.168736, 128.0,
        ]);
        static V_COEFF: f32x16 = f32x16::from_array([
            -0.081312, -0.418688, 0.500000, 128.0, -0.081312, -0.418688, 0.500000, 128.0,
            -0.081312, -0.418688, 0.500000, 128.0, -0.081312, -0.418688, 0.500000, 128.0,
        ]);

        let pixels_count = bgra_pixels.len() / 4;
        let chunks_count = pixels_count / 4;

        let start = Instant::now();

        (0..chunks_count)
            .map(|i| {
                let bgra_4_pixels = u8x16::from_slice(&bgra_pixels[i * 16..i * 16 + 16]);
                let bgra_4_pixels: f32x16 = bgra_4_pixels.cast();
                (i, bgra_4_pixels)
            })
            .map(|(i, bgra_4_pixels)| {
                let y_4_values = bgra_4_pixels * Y_COEFF;
                let u_4_values = bgra_4_pixels * U_COEFF;
                let v_4_values = bgra_4_pixels * V_COEFF;

                let y0 = simd_swizzle!(y_4_values, [0, 1, 2, 3]).horizontal_sum();
                let y1 = simd_swizzle!(y_4_values, [4, 5, 6, 7]).horizontal_sum();
                let y2 = simd_swizzle!(y_4_values, [8, 9, 10, 11]).horizontal_sum();
                let y3 = simd_swizzle!(y_4_values, [12, 13, 14, 15]).horizontal_sum();

                let u = u_4_values.horizontal_sum();
                let v = v_4_values.horizontal_sum();

                (i, (y0, y1, y2, y3, u, v))
            })
            .for_each(|(i, (y0, y1, y2, y3, u, v))| {
                y_pixels[i] = clamp(y0);
                y_pixels[i + 1] = clamp(y1);
                y_pixels[i + 2] = clamp(y2);
                y_pixels[i + 3] = clamp(y3);

                u_pixels[i / 4] = clamp(u * 0.25);
                v_pixels[i / 4] = clamp(v * 0.25);
            });

        debug!("Time: {}", start.elapsed().as_millis());
    }
}
