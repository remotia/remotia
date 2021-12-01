extern crate scrap;

use std::time::Duration;

use clap::Parser;
use remotia::{
    common::command_line::parse_canvas_resolution_str,
    server::{
        capture::scrap::ScrapFrameCapturer,
        encode::ffmpeg::h264::H264Encoder,
        pipeline::silo::{SiloServerConfiguration, SiloServerPipeline},
        send::srt::SRTFrameSender,
    },
};

#[derive(Parser)]
#[clap(version = "0.1.0", author = "Lorenzo C. <aegroto@protonmail.com>")]
pub struct CommandLineServerOptions {
    #[clap(short, long, default_value = "1280x720")]
    resolution: String,

    #[clap(long)]
    console_profiling: bool,

    #[clap(long)]
    csv_profiling: bool,

    #[clap(long, default_value = "100")]
    latency: u64,

    #[clap(long, default_value = "15")]
    timeout: u64,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let options = CommandLineServerOptions::parse();
    let (width, height) = parse_canvas_resolution_str(&options.resolution);

    let frame_size = width * height * 3;

    let srt_sender = Box::new(
        SRTFrameSender::new(
            5001,
            Duration::from_millis(options.latency),
            Duration::from_millis(options.timeout),
        )
        .await,
    );

    let pipeline = SiloServerPipeline::new(SiloServerConfiguration {
        frame_capturer: Box::new(ScrapFrameCapturer::new_from_primary()),
        encoder: Box::new(H264Encoder::new(
            frame_size as usize,
            width as i32,
            height as i32,
        )),
        frame_sender: srt_sender,
        console_profiling: options.console_profiling,
        csv_profiling: options.csv_profiling,

        width: width as usize,
        height: height as usize
    });

    pipeline.run().await;

    Ok(())
}
