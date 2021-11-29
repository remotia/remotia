extern crate scrap;

use clap::Parser;
use remotia::{common::command_line::parse_canvas_resolution_str, server::{ServerConfiguration, encode::ffmpeg::h264::H264Encoder, run_with_configuration, send::srt::SRTFrameSender}};

#[derive(Parser)]
#[clap(version = "0.1.0", author = "Lorenzo C. <aegroto@protonmail.com>")]
pub struct CommandLineServerOptions {
    #[clap(short, long, default_value = "1280x720")]
    resolution: String,

    #[clap(long)]
    console_profiling: bool,

    #[clap(long)]
    csv_profiling: bool,
}

fn main() -> std::io::Result<()> {
    env_logger::init();
    let options = CommandLineServerOptions::parse();
    let (width, height) = parse_canvas_resolution_str(&options.resolution);

    let frame_size = width * height * 3;

    run_with_configuration(ServerConfiguration {
        encoder: Box::new(H264Encoder::new(
            frame_size as usize,
            width as i32,
            height as i32,
        )),
        frame_sender: Box::new(SRTFrameSender::new()),
        console_profiling: options.console_profiling,
        csv_profiling: options.csv_profiling,
    })
}
