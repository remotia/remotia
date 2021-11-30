extern crate scrap;

mod utils;

use clap::Parser;
use remotia::{common::command_line::parse_canvas_resolution_str, server::{capture::scrap::ScrapFrameCapturer, pipeline::waterfall::{WaterfallPipeline, WaterfallServerConfiguration}}};
use utils::{setup_encoder_by_name, setup_frame_sender_by_name};

#[derive(Parser)]
#[clap(version = "0.1.0", author = "Lorenzo C. <aegroto@protonmail.com>")]
pub struct CommandLineServerOptions {
    #[clap(short, long, default_value = "h264rgb")]
    encoder_name: String,

    #[clap(short, long, default_value = "tcp")]
    frame_sender_name: String,

    #[clap(short, long, default_value = "1280x720")]
    resolution: String,

    #[clap(long)]
    console_profiling: bool,

    #[clap(long)]
    csv_profiling: bool,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let options = CommandLineServerOptions::parse();
    let (width, height) = parse_canvas_resolution_str(&options.resolution);

    let pipeline = WaterfallPipeline::new(WaterfallServerConfiguration {
        frame_capturer: Box::new(ScrapFrameCapturer::new_from_primary()),
        encoder: setup_encoder_by_name(width as usize, height as usize, &options.encoder_name),
        frame_sender: setup_frame_sender_by_name(&options.frame_sender_name)?,
        console_profiling: options.console_profiling,
        csv_profiling: options.csv_profiling,
    });

    pipeline.run().await;

    Ok(())
}
