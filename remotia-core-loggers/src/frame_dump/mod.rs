use async_trait::async_trait;

use jpegxl_rs::{
    encode::{EncoderSpeed, JxlEncoder},
    encoder_builder,
};
use log::info;
use remotia::{traits::FrameProcessor, types::FrameData};

pub struct RawFrameDumper<'prl, 'mm> {
    buffer_id: String,

    width: u32,
    height: u32,

    folder: String
}

impl<'prl, 'mm> RawFrameDumper<'prl, 'mm> {
    pub fn new(buffer_id: &str, width: usize, height: usize, folder: &str) -> Self {
        let encoder = encoder_builder()
            .lossless(true)
            .speed(EncoderSpeed::Lightning)
            .build()
            .unwrap();

        let internal_rgb_buffer = vec![0; width * height * 3];
        let buffer_id = buffer_id.to_string();

        let width = width as u32;
        let height = height as u32;

        Self {
            encoder,
            buffer_id,
            internal_rgb_buffer,
            width,
            height,
            folder: folder.to_string()
        }
    }

    fn fill_rgb_buffer(&mut self, frame_data: &mut FrameData) {
        let bgra_buffer = frame_data.get_writable_buffer_ref(&self.buffer_id).unwrap();
        let pixels_count = (self.width * self.height) as usize;

        for i in 0..pixels_count {
            let (b, g, r) = (
                bgra_buffer[i * 4],
                bgra_buffer[i * 4 + 1],
                bgra_buffer[i * 4 + 2],
            );

            self.internal_rgb_buffer[i * 3] = r;
            self.internal_rgb_buffer[i * 3 + 1] = g;
            self.internal_rgb_buffer[i * 3 + 2] = b;
        }
    }
}

#[async_trait]
impl<'prl, 'mm> FrameProcessor for RawFrameDumper<'prl, 'mm> {
    async fn process(&mut self, mut frame_data: FrameData) -> Option<FrameData> {
        let frame_id = frame_data.get("capture_timestamp");
        /*self.fill_rgb_buffer(&mut frame_data);
        let encode_result = self.encoder
            .encode::<u8, u8>(&self.internal_rgb_buffer, self.width, self.height);*/


        Some(frame_data)
    }
}
