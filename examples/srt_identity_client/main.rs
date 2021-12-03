use std::time::Duration;

use clap::Parser;
use remotia::{
    client::{
        decode::{h264::H264Decoder, identity::IdentityDecoder},
        pipeline::waterfall::{WaterfallClientConfiguration, WaterfallClientPipeline},
        receive::srt::SRTFrameReceiver,
    },
    common::command_line::parse_canvas_resolution_str,
};

#[derive(Parser)]
#[clap(version = "0.1.0", author = "Lorenzo C. <aegroto@protonmail.com>")]
struct Options {
    #[clap(short, long, default_value = "1280x720")]
    resolution: String,

    #[clap(short, long, default_value = "127.0.0.1:5001")]
    server_address: String,

    #[clap(short, long, default_value = "5002")]
    binding_port: String,

    #[clap(short, long, default_value = "100")]
    maximum_consecutive_connection_losses: u32,

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
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let options = Options::parse();
    let (canvas_width, canvas_height) = parse_canvas_resolution_str(&options.resolution);

    let receiver = Box::new(
        SRTFrameReceiver::new(
            &options.server_address,
            Duration::from_millis(options.latency),
            Duration::from_millis(options.timeout),
        )
        .await,
    );

    let pipeline = WaterfallClientPipeline::new(WaterfallClientConfiguration {
        decoder: Box::new(IdentityDecoder::new(
            canvas_width as usize,
            canvas_height as usize,
        )),
        frame_receiver: receiver,
        canvas_width: canvas_width,
        canvas_height: canvas_height,
        maximum_consecutive_connection_losses: options.maximum_consecutive_connection_losses,
        console_profiling: options.console_profiling,
        csv_profiling: options.csv_profiling,
    });
    pipeline.run().await;
    Ok(())
}
