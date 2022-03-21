use beryllium::{
    gl_window::{GlAttr, GlContextFlags, GlProfile, GlWindow},
    init::{InitFlags, Sdl},
    window::WindowFlags,
};
use pixels::{Pixels, PixelsBuilder, SurfaceTexture};
use remotia_core::{traits::FrameProcessor, types::FrameData};
use zstring::zstr;

use async_trait::async_trait;

pub struct BerylliumRenderer {
    _gl_win: GlWindow,
    pixels: Pixels,
}
unsafe impl Send for BerylliumRenderer {}

impl BerylliumRenderer {
    pub fn new(canvas_width: u32, canvas_height: u32) -> Self {
        // Init display
        let gl_win = create_gl_window(canvas_width as i32, canvas_height as i32);
        let window = &*gl_win;

        let pixels = {
            let surface_texture = SurfaceTexture::new(canvas_width, canvas_height, &window);
            PixelsBuilder::new(canvas_width, canvas_height, surface_texture)
                .build()
                .unwrap()
        };

        Self {
            _gl_win: gl_win,
            pixels
        }
    }
}

#[async_trait]
impl FrameProcessor for BerylliumRenderer {
    async fn process(&mut self, mut frame_data: FrameData) -> Option<FrameData> {
        let raw_frame_buffer = frame_data.get_writable_buffer_ref("raw_frame_buffer").unwrap();
        packed_bgra_to_packed_rgba(raw_frame_buffer, self.pixels.get_frame());
        self.pixels.render().unwrap();

        Some(frame_data)
    }
}

pub fn create_gl_window(width: i32, height: i32) -> GlWindow {
    let sdl = Sdl::init(InitFlags::EVERYTHING).unwrap();
    sdl.allow_drop_events(true);

    const FLAGS: i32 = if cfg!(debug_assertions) {
        GlContextFlags::FORWARD_COMPATIBLE.as_i32() | GlContextFlags::DEBUG.as_i32()
    } else {
        GlContextFlags::FORWARD_COMPATIBLE.as_i32()
    };
    sdl.gl_set_attribute(GlAttr::MajorVersion, 3).unwrap();
    sdl.gl_set_attribute(GlAttr::MinorVersion, 3).unwrap();
    sdl.gl_set_attribute(GlAttr::Profile, GlProfile::Core as _)
        .unwrap();
    sdl.gl_set_attribute(GlAttr::Flags, FLAGS).unwrap();

    let gl_win = sdl
        .create_gl_window(
            zstr!("Remotia client"),
            None,
            (width, height),
            WindowFlags::ALLOW_HIGHDPI,
        )
        .unwrap();
    gl_win.set_swap_interval(1).unwrap();

    gl_win
}

pub fn packed_bgr_to_packed_rgba(packed_bgr_buffer: &[u8], packed_rgba_buffer: &mut [u8]) {
    let pixels_count = packed_rgba_buffer.len() / 4;

    for i in 0..pixels_count {
        packed_rgba_buffer[i * 4 + 2] = packed_bgr_buffer[i * 3];
        packed_rgba_buffer[i * 4 + 1] = packed_bgr_buffer[i * 3 + 1];
        packed_rgba_buffer[i * 4] = packed_bgr_buffer[i * 3 + 2];
    }
}

pub fn packed_bgra_to_packed_rgba(packed_bgra_buffer: &[u8], packed_rgba_buffer: &mut [u8]) {
    let pixels_count = packed_rgba_buffer.len() / 4;

    for i in 0..pixels_count {
        // BGR -> RGB channels
        packed_rgba_buffer[i * 4] = packed_bgra_buffer[i * 4 + 2];
        packed_rgba_buffer[i * 4 + 1] = packed_bgra_buffer[i * 4 + 1];
        packed_rgba_buffer[i * 4 + 2] = packed_bgra_buffer[i * 4];

        // Alpha channel
        packed_rgba_buffer[i * 4 + 3] = packed_bgra_buffer[i * 4 + 3];
    }
}
