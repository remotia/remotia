extern crate scrap;

use clap::Parser;
use remotia::{
    common::command_line::parse_canvas_resolution_str,
    server::{
        capture::scrap::ScrapFrameCapturer,
        encode::ffmpeg::h264::H264Encoder,
        pipeline::silo::{BuffersConfig, SiloServerConfiguration, SiloServerPipeline},
        profiling::{console::ConsoleServerProfiler, tcp::TCPServerProfiler, ServerProfiler},
        send::remvsp::{RemVPSFrameSenderConfiguration, RemVSPFrameSender},
    },
};

use remotia::server::encode::pool::PoolEncoder;

#[derive(Parser)]
#[clap(version = "0.1.0", author = "Lorenzo C. <aegroto@protonmail.com>")]
pub struct CommandLineServerOptions {
    #[clap(short, long, default_value = "1280x720")]
    resolution: String,

    #[clap(long)]
    console_profiling: bool,

    #[clap(long)]
    csv_profiling: bool,

    #[clap(long, default_value = "60")]
    frames_capture_rate: u32,

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

    let profilers: Vec<Box<dyn ServerProfiler + Send>> = vec![
        Box::new(TCPServerProfiler::connect()),
        Box::new(ConsoleServerProfiler {
            header: Some(String::from("Average computational times")),
            values_to_log: vec![
                String::from("capture_time"),
                String::from("encoding_time"),
                String::from("transfer_time"),
            ],

            ..Default::default()
        }),
    ];

    let buffer_size = (width * height * 4) as usize;
    let encoder = Box::new(PoolEncoder::new(
        buffer_size,
        vec![
            Box::new(H264Encoder::new(buffer_size, width as i32, height as i32)),
            Box::new(H264Encoder::new(buffer_size, width as i32, height as i32)),
            Box::new(H264Encoder::new(buffer_size, width as i32, height as i32)),
            Box::new(H264Encoder::new(buffer_size, width as i32, height as i32)),
        ],
    ));

    let frame_sender = Box::new(RemVSPFrameSender::listen(
        5001,
        512,
        RemVPSFrameSenderConfiguration {
            retransmission_frequency: 0.5,
        },
    ));

    let pipeline = SiloServerPipeline::new(SiloServerConfiguration {
        frame_capturer: Box::new(ScrapFrameCapturer::new_from_primary()),
        encoder: encoder,
        frame_sender: frame_sender,
        profilers: profilers,

        frames_capture_rate: options.frames_capture_rate,

        width: width as usize,
        height: height as usize,
        maximum_preencoding_capture_delay: 5,
        buffers_conf: BuffersConfig {
            maximum_raw_frame_buffers: 4,
            maximum_encoded_frame_buffers: 256,
        },
    });

    pipeline.run().await;

    Ok(())
}
