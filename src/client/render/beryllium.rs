use beryllium::{
    event::Event,
    gl_window::{GlAttr, GlContextFlags, GlProfile, GlWindow},
    init::{InitFlags, Sdl},
    window::WindowFlags,
    SdlResult,
};
use log::debug;
use pixels::{Pixels, PixelsBuilder, SurfaceTexture};
use zstring::zstr;

use super::Renderer;

pub struct BerylliumRenderer {
    _gl_win: GlWindow,
    pixels: Pixels,

    canvas_width: u32,
    canvas_height: u32,
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
            pixels,
            canvas_width,
            canvas_height,
        }
    }
}

impl Renderer for BerylliumRenderer {
    fn render(&mut self, raw_frame_buffer: &[u8]) {
        packed_bgr_to_packed_rgba(&raw_frame_buffer, self.pixels.get_frame());
        self.pixels.render().unwrap();
    }

    fn handle_feedback(&mut self, message: crate::common::feedback::FeedbackMessage) {
        debug!("Feedback message: {:?}", message);
    }

    fn get_buffer_size(&self) -> usize {
        (self.canvas_width * self.canvas_height * 3) as usize
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

pub fn packed_bgr_to_packed_rgba(packed_bgr_buffer: &[u8], packed_bgra_buffer: &mut [u8]) {
    let pixels_count = packed_bgra_buffer.len() / 4;

    for i in 0..pixels_count {
        packed_bgra_buffer[i * 4 + 2] = packed_bgr_buffer[i * 3];
        packed_bgra_buffer[i * 4 + 1] = packed_bgr_buffer[i * 3 + 1];
        packed_bgra_buffer[i * 4] = packed_bgr_buffer[i * 3 + 2];
    }
}