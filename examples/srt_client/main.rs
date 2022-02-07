use clap::Parser;
use remotia::client::decode::pool::PoolDecoder;
use remotia::client::receive::srt::SRTFrameReceiver;
use remotia::{
    client::{
        decode::h264::H264Decoder,
        pipeline::silo::{BuffersConfig, SiloClientConfiguration, SiloClientPipeline},
        profiling::tcp::TCPClientProfiler,
        render::beryllium::BerylliumRenderer,
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

    #[clap(short, long, default_value = "60")]
    frames_render_rate: u32,

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

    let decoder = Box::new(PoolDecoder::new(vec![
        Box::new(H264Decoder::new()),
        Box::new(H264Decoder::new()),
        Box::new(H264Decoder::new()),
        Box::new(H264Decoder::new()),
    ]));

    let frame_receiver = Box::new(SRTFrameReceiver::new(&options.server_address).await);

    let profiler = Box::new(TCPClientProfiler::connect().await);

    let pipeline = SiloClientPipeline::new(SiloClientConfiguration {
        decoder,
        frame_receiver,
        renderer,

        profiler,

        maximum_consecutive_connection_losses: options.maximum_consecutive_connection_losses,
        frames_render_rate: options.frames_render_rate,
        console_profiling: options.console_profiling,
        csv_profiling: options.csv_profiling,

        buffers_conf: BuffersConfig {
            maximum_encoded_frame_buffers: 32,
            maximum_raw_frame_buffers: 32,
        },

        maximum_pre_render_frame_delay: 30000,
    });

    pipeline.run().await;

    Ok(())
}
