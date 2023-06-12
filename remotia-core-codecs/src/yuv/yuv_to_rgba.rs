use async_trait::async_trait;
use bytes::BytesMut;
use remotia_core::traits::{FrameProcessor, PullableFrameProperties};

use super::yuv_to_rgb;

pub struct YUV420PToRGBAConverter<K: Copy> {
    stride: usize,

    r_offset: usize,
    g_offset: usize,
    b_offset: usize,

    rgba_buffer_key: K,
    y_buffer_key: K,
    cb_buffer_key: K,
    cr_buffer_key: K,
}

impl<K: Copy> YUV420PToRGBAConverter<K> {
    pub fn new(stride: usize, y_buffer_key: K, cb_buffer_key: K, cr_buffer_key: K, rgba_buffer_key: K) -> Self {
        Self {
            stride, 

            r_offset: 0,
            g_offset: 0,
            b_offset: 0,

            rgba_buffer_key,
            y_buffer_key,
            cb_buffer_key,
            cr_buffer_key,
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

        let pixels_count = rgba_buffer.len() / 4; 
        let lines = pixels_count / self.stride; 

        // let (r, g, b) = yuv_to_rgb(y, u, v);
        for h in 0..lines {
            for w in 0..self.stride {
            }
        }

        frame_data.push(self.rgba_buffer_key, rgba_buffer);
        frame_data.push(self.y_buffer_key, y_buffer);
        frame_data.push(self.cb_buffer_key, cb_buffer);
        frame_data.push(self.cr_buffer_key, cr_buffer);

        Some(frame_data)
    }
}
