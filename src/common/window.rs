use beryllium::{
    event::Event,
    gl_window::{GlAttr, GlContextFlags, GlProfile, GlWindow},
    init::{InitFlags, Sdl},
    window::WindowFlags,
    SdlResult,
};
use zstring::zstr;

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
