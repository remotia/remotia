use async_trait::async_trait;
use bytes::BytesMut;
use remotia_core::traits::{FrameProcessor, PullableFrameProperties};

use super::bgr_to_yuv_f32;

pub struct RGBAToYUV420PConverter<K: Copy> {
    rgba_buffer_key: K,
    y_buffer_key: K,
    cb_buffer_key: K,
    cr_buffer_key: K,
}

impl<K: Copy> RGBAToYUV420PConverter<K> {
    pub fn new(rgba_buffer_key: K, y_buffer_key: K, cb_buffer_key: K, cr_buffer_key: K) -> Self {
        Self {
            rgba_buffer_key,
            y_buffer_key,
            cb_buffer_key,
            cr_buffer_key,
        }
    }

    fn convert(
        &self,
        bgra_pixels: &[u8],
        y_pixels: &mut [u8],
        u_pixels: &mut [u8],
        v_pixels: &mut [u8],
    ) {
        u_pixels.fill(0);
        v_pixels.fill(0);
        for i in 0..y_pixels.len() {
            let (b, g, r) = (
                bgra_pixels[i * 4],
                bgra_pixels[i * 4 + 1],
                bgra_pixels[i * 4 + 2],
            );
            let (y, u, v) = bgr_to_yuv_f32(b, g, r);

            y_pixels[i] = y as u8;
            u_pixels[i / 4] += u as u8 / 4;
            v_pixels[i / 4] += v as u8 / 4;
        }
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

        self.convert(&rgba_buffer, &mut y_buffer, &mut cb_buffer, &mut cr_buffer);

        frame_data.push(self.rgba_buffer_key, rgba_buffer);
        frame_data.push(self.y_buffer_key, y_buffer);
        frame_data.push(self.cb_buffer_key, cb_buffer);
        frame_data.push(self.cr_buffer_key, cr_buffer);

        Some(frame_data)
    }
}
