use std::{net::SocketAddr, str::FromStr};

use clap::Parser;
use remotia::{client::{pipeline::silo::{SiloClientConfiguration, SiloClientPipeline}, profiling::tcp::TCPClientProfiler, render::beryllium::BerylliumRenderer}, common::{
        command_line::parse_canvas_resolution_str,
        helpers::client_setup::{setup_decoder_from_name, setup_frame_receiver_by_name},
    }};

#[derive(Parser)]
#[clap(version = "0.1.0", author = "Lorenzo C. <aegroto@protonmail.com>")]
struct Options {
    #[clap(short, long, default_value = "h264")]
    decoder_name: String,

    #[clap(short, long, default_value = "remvsp")]
    frame_receiver_name: String,

    #[clap(short, long, default_value = "1280x720")]
    resolution: String,

    #[clap(short, long, default_value = "127.0.0.1:5001")]
    server_address: String,

    #[clap(short, long, default_value = "5002")]
    binding_port: String,

    #[clap(short, long, default_value = "100")]
    maximum_consecutive_connection_losses: u32,

    #[clap(short, long, default_value = "60")]
    target_fps: u32,

    #[clap(long)]
    console_profiling: bool,

    #[clap(long)]
    csv_profiling: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let options = Options::parse();
    let (canvas_width, canvas_height) = parse_canvas_resolution_str(&options.resolution);

    let renderer = Box::new(BerylliumRenderer::new(canvas_width, canvas_height));

    let decoder = setup_decoder_from_name(canvas_width, canvas_height, &options.decoder_name);
    let frame_receiver = setup_frame_receiver_by_name(
        SocketAddr::from_str(&options.server_address)?,
        &options.binding_port,
        &options.frame_receiver_name,
    )
    .await
    .unwrap();
    let profiler = Box::new(TCPClientProfiler::connect().await);

    let pipeline = SiloClientPipeline::new(SiloClientConfiguration {
        decoder,
        frame_receiver,
        renderer,

        profiler,

        maximum_consecutive_connection_losses: options.maximum_consecutive_connection_losses,
        target_fps: options.target_fps,
        console_profiling: options.console_profiling,
        csv_profiling: options.csv_profiling,
    });

    pipeline.run().await;

    Ok(())
}
