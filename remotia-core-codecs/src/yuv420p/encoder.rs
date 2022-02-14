use async_trait::async_trait;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use remotia::{traits::FrameProcessor, types::FrameData};

pub struct RGBAToYUV420PConverter {}

impl RGBAToYUV420PConverter {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl FrameProcessor for RGBAToYUV420PConverter {
    async fn process(&mut self, mut frame_data: FrameData) -> Option<FrameData> {
        let mut raw_frame_buffer = frame_data
            .extract_writable_buffer("raw_frame_buffer")
            .unwrap();
        let mut y_channel_buffer = frame_data
            .extract_writable_buffer("y_channel_buffer")
            .unwrap();
        let mut cb_channel_buffer = frame_data
            .extract_writable_buffer("cb_channel_buffer")
            .unwrap();
        let mut cr_channel_buffer = frame_data
            .extract_writable_buffer("cr_channel_buffer")
            .unwrap();

        bgra_to_yuv_separate(
            &mut raw_frame_buffer,
            &mut y_channel_buffer,
            &mut cb_channel_buffer,
            &mut cr_channel_buffer,
        );

        frame_data.insert_writable_buffer("raw_frame_buffer", raw_frame_buffer);
        frame_data.insert_writable_buffer("y_channel_buffer", y_channel_buffer);
        frame_data.insert_writable_buffer("cb_channel_buffer", cb_channel_buffer);
        frame_data.insert_writable_buffer("cr_channel_buffer", cr_channel_buffer);

        Some(frame_data)
    }
}

fn bgra_to_yuv_separate(
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

            (i, bgr_to_yuv_f32(b, g, r))
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

pub fn bgr_to_yuv_f32(_b: u8, _g: u8, _r: u8) -> (f32, f32, f32) {
    let r = _r as f32;
    let g = _g as f32;
    let b = _b as f32;

    let y = r * 0.29900 + g * 0.58700 + b * 0.11400;
    let u = (r * -0.16874 + g * -0.33126 + b * 0.50000) + 128.0;
    let v = (r * 0.50000 + g * -0.41869 + b * -0.08131) + 128.0;

    (y, u, v)
}
