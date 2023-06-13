use pixels::{Pixels, PixelsBuilder, SurfaceTexture};
use remotia_buffer_utils::BytesMut;
use remotia_core::traits::{BorrowMutFrameProperties, FrameProcessor};

use async_trait::async_trait;
use winit::{
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

pub struct WinitRenderer<K> {
    buffer_key: K,
    pixels: Pixels,
    _window: Window,
}
unsafe impl<K> Send for WinitRenderer<K> {}

impl<K> WinitRenderer<K> {
    pub fn new(buffer_key: K, canvas_width: u32, canvas_height: u32) -> Self {
        let window = WindowBuilder::new().build(&EventLoop::new()).unwrap();

        let pixels = {
            let surface_texture = SurfaceTexture::new(canvas_width, canvas_height, &window);
            PixelsBuilder::new(canvas_width, canvas_height, surface_texture)
                .build()
                .unwrap()
        };

        Self {
            buffer_key,
            pixels,
            _window: window,
        }
    }
}

#[async_trait]
impl<F, K> FrameProcessor<F> for WinitRenderer<K>
where
    F: BorrowMutFrameProperties<K, BytesMut> + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        let raw_frame_buffer = frame_data.get_mut_ref(&self.buffer_key).unwrap();
        self.pixels.frame_mut().copy_from_slice(raw_frame_buffer);
        self.pixels.render().unwrap();

        Some(frame_data)
    }
}
