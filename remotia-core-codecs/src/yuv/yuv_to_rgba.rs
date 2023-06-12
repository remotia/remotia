use async_trait::async_trait;
use bytes::BytesMut;
use remotia_core::traits::{FrameProcessor, PullableFrameProperties};

use super::yuv_to_rgb;

pub struct YUV420PToRGBAConverter<K: Copy> {
    rgba_buffer_key: K,
    y_buffer_key: K,
    cb_buffer_key: K,
    cr_buffer_key: K,
}

impl<K: Copy> YUV420PToRGBAConverter<K> {
    pub fn new(y_buffer_key: K, cb_buffer_key: K, cr_buffer_key: K, rgba_buffer_key: K) -> Self {
        Self {
            rgba_buffer_key,
            y_buffer_key,
            cb_buffer_key,
            cr_buffer_key,
        }
    }

    pub fn convert(
        &self,
        y_pixels: &[u8],
        u_pixels: &[u8],
        v_pixels: &[u8],
        rgba_pixels: &mut [u8],
    ) {
        for i in 0..y_pixels.len() {
            let (y, u, v) = (y_pixels[i], u_pixels[i / 4], v_pixels[i / 4]);
            let (r, g, b) = yuv_to_rgb(y, u, v);
            rgba_pixels[i * 4] = r;
            rgba_pixels[i * 4 + 1] = g;
            rgba_pixels[i * 4 + 2] = b;
        }
    }
}

#[async_trait]
impl<F, K> FrameProcessor<F> for YUV420PToRGBAConverter<K>
where
    K: Copy + Send,
    F: PullableFrameProperties<K, BytesMut> + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        let y_buffer = frame_data.pull(&self.y_buffer_key).unwrap();
        let cb_buffer = frame_data.pull(&self.cb_buffer_key).unwrap();
        let cr_buffer = frame_data.pull(&self.cr_buffer_key).unwrap();
        let mut rgba_buffer = frame_data.pull(&self.rgba_buffer_key).unwrap();

        self.convert(&y_buffer, &cb_buffer, &cr_buffer, &mut rgba_buffer);

        frame_data.push(self.rgba_buffer_key, rgba_buffer);
        frame_data.push(self.y_buffer_key, y_buffer);
        frame_data.push(self.cb_buffer_key, cb_buffer);
        frame_data.push(self.cr_buffer_key, cr_buffer);

        Some(frame_data)
    }
}
