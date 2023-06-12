use async_trait::async_trait;
use bytes::{BytesMut, BufMut};
use remotia_core::traits::{FrameProcessor, PullableFrameProperties};

use super::bgr_to_yuv_f32;

pub struct RGBAToYUV420PConverter<K: Copy> {
    stride: usize,

    r_offset: usize,
    g_offset: usize,
    b_offset: usize,

    rgba_buffer_key: K,
    y_buffer_key: K,
    cb_buffer_key: K,
    cr_buffer_key: K,
}


impl<K: Copy> RGBAToYUV420PConverter<K> {
    pub fn new(stride: usize, rgba_buffer_key: K, y_buffer_key: K, cb_buffer_key: K, cr_buffer_key: K) -> Self {
        Self {
            r_offset: 0,
            g_offset: 1,
            b_offset: 2,

            stride,
            rgba_buffer_key,
            y_buffer_key,
            cb_buffer_key,
            cr_buffer_key,
        }
    }

    pub fn pixel_at(&self, pixels: &[u8], h: usize, w: usize) -> (u8, u8, u8) {
        let p = (h * self.stride + w) * 4;
        (pixels[p + self.r_offset], pixels[p + self.g_offset], pixels[p + self.b_offset])
    }
}


#[async_trait]
impl<F, K> FrameProcessor<F> for RGBAToYUV420PConverter<K>
where
    K: Copy + Send,
    F: PullableFrameProperties<K, BytesMut> + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        let rgba_buffer = frame_data.pull(&self.rgba_buffer_key).unwrap();
        let mut y_buffer = frame_data.pull(&self.y_buffer_key).unwrap();
        let mut cb_buffer = frame_data.pull(&self.cb_buffer_key).unwrap();
        let mut cr_buffer = frame_data.pull(&self.cr_buffer_key).unwrap();

        let pixels_count = rgba_buffer.len() / 4; 
        let lines = pixels_count / self.stride; 
        
        for h in 0..lines / 2 {
            for w in 0..self.stride / 2 {
                let (y0, u0, v0) = bgr_to_yuv_f32(self.pixel_at(&rgba_buffer, h * 2, w * 2));
                let (y1, u1, v1) = bgr_to_yuv_f32(self.pixel_at(&rgba_buffer, h * 2 + 1, w * 2));
                let (y2, u2, v2) = bgr_to_yuv_f32(self.pixel_at(&rgba_buffer, h * 2, w * 2 + 1));
                let (y3, u3, v3) = bgr_to_yuv_f32(self.pixel_at(&rgba_buffer, h * 2 + 1, w * 2 + 1));

                y_buffer.put_u8(y0 as u8);
                y_buffer.put_u8(y1 as u8);
                y_buffer.put_u8(y2 as u8);
                y_buffer.put_u8(y3 as u8);

                cb_buffer.put_u8(((u0 + u1 + u2 + u3) / 4.0) as u8);
                cr_buffer.put_u8(((v0 + v1 + v2 + v3) / 4.0) as u8);
            }
        }

        frame_data.push(self.rgba_buffer_key, rgba_buffer);
        frame_data.push(self.y_buffer_key, y_buffer);
        frame_data.push(self.cb_buffer_key, cb_buffer);
        frame_data.push(self.cr_buffer_key, cr_buffer);

        Some(frame_data)
    }
}
