extern crate scrap;

use std::{net::TcpListener, time::Duration};

use clap::Parser;
use log::info;
use remotia::{
    common::{
        command_line::parse_canvas_resolution_str,
        helpers::server_setup::{setup_encoder_by_name, setup_frame_sender_by_name},
    },
    server::{
        capture::scrap::ScrapFrameCapturer,
        encode::ffmpeg::h264::H264Encoder,
        pipeline::silo::{SiloServerConfiguration, SiloServerPipeline},
        send::{srt::SRTFrameSender, tcp::TCPFrameSender},
    },
};

#[derive(Parser)]
#[clap(version = "0.1.0", author = "Lorenzo C. <aegroto@protonmail.com>")]
pub struct CommandLineServerOptions {
    #[clap(short, long, default_value = "1280x720")]
    resolution: String,

    #[clap(short, long, default_value = "h264")]
    encoder_name: String,

    #[clap(short, long, default_value = "srt")]
    frame_sender_name: String,

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

    let encoder = setup_encoder_by_name(width as usize, height as usize, &options.encoder_name);
    let frame_sender = setup_frame_sender_by_name(&options.frame_sender_name)
        .await
        .unwrap();

    let pipeline = SiloServerPipeline::new(SiloServerConfiguration {
        frame_capturer: Box::new(ScrapFrameCapturer::new_from_primary()),
        encoder: encoder,
        frame_sender: frame_sender,
        console_profiling: options.console_profiling,
        csv_profiling: options.csv_profiling,

        width: width as usize,
        height: height as usize,
    });

    pipeline.run().await;

    Ok(())
}
