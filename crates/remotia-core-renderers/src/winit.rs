use std::{
    future::Future,
    sync::{Arc, Mutex},
};

use pixels::{Pixels, SurfaceTexture};
use remotia_buffer_utils::BytesMut;
use remotia_core::traits::{BorrowMutFrameProperties, FrameProcessor};

use async_trait::async_trait;
use winit::{
    dpi::LogicalSize,
    event_loop::{self, EventLoop},
    window::WindowBuilder,
};

pub struct WinitRenderer<'a, K> {
    buffer_key: K,
    pixels: Option<Arc<Mutex<Pixels<'a>>>>,
}

impl<'a, K> WinitRenderer<'a, K> {
    pub fn new(buffer_key: K) -> Self {
        Self {
            buffer_key,
            pixels: None,
        }
    }

    pub fn allocate(&mut self, width: u32, height: u32) -> WinitRunner<'a> {
        let event_loop = EventLoop::new().unwrap();

        let window = { WindowBuilder::new().build(&event_loop).unwrap() };

        let pixels = Arc::new(Mutex::new({
            let surface_texture = SurfaceTexture::new(width, height, window);
            Pixels::new(width, height, surface_texture).unwrap()
        }));

        self.pixels = Some(pixels.clone());

        WinitRunner { event_loop, pixels }
    }
}

pub struct WinitRunner<'a> {
    event_loop: EventLoop<()>,
    pixels: Arc<Mutex<Pixels<'a>>>,
}

unsafe impl<'a> Send for WinitRunner<'a> {}

impl<'a> WinitRunner<'a> {
    pub fn start(self) {
        self.event_loop
            .run(|_event, _elwt| {
                self.pixels.lock().unwrap().render().unwrap();
                log::debug!("Rendering buffer...")
            })
            .unwrap();
    }
}

#[async_trait]
impl<'a, F, K> FrameProcessor<F> for WinitRenderer<'a, K>
where
    K: Send,
    F: BorrowMutFrameProperties<K, BytesMut> + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        log::debug!("Filling pixels buffer...");

        let raw_frame_buffer = frame_data.get_mut_ref(&self.buffer_key).unwrap();
        let mut pixels = self.pixels.as_mut().unwrap().lock().unwrap();
        pixels.frame_mut().copy_from_slice(raw_frame_buffer);

        Some(frame_data)
    }
}
