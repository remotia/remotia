use std::time::Duration;

use remotia::{
    processors::{
        error_switch::OnErrorSwitch, frame_drop::ThresholdBasedFrameDropper, ticker::Ticker,
    },
    server::pipeline::ascode::{component::Component, AscodePipeline},
};
use remotia_buffer_utils::BufferAllocator;
use remotia_core_capturers::scrap::ScrapFrameCapturer;
use remotia_core_loggers::{printer::ConsoleFrameDataPrinter, stats::ConsoleAverageStatsLogger};
use remotia_ffmpeg_codecs::encoders::h264::H264Encoder;
use remotia_profilation_utils::time::{add::TimestampAdder, diff::TimestampDiffCalculator};
use remotia_srt::sender::SRTFrameSender;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let error_handling_pipeline = AscodePipeline::new()
        .tag("ErrorsHandler")
        .link(Component::new().add(ConsoleFrameDataPrinter::new()))
        .bind()
        .feedable();

    let capturer = ScrapFrameCapturer::new_from_primary();
    let width = capturer.width();
    let height = capturer.height();
    let buffer_size = width * height * 4;

    let main_pipeline = AscodePipeline::new()
        .tag("ServerMain")
        .link(
            Component::new()
                .add(Ticker::new(1000))
                .add(TimestampAdder::new("process_start_timestamp"))
                .add(BufferAllocator::new("raw_frame_buffer", buffer_size))
                .add(TimestampAdder::new("capture_timestamp"))
                .add(capturer),
        )
        .link(
            Component::new()
                .add(TimestampDiffCalculator::new(
                    "capture_timestamp",
                    "capture_delay",
                ))
                .add(ThresholdBasedFrameDropper::new("capture_delay", 10))
                .add(OnErrorSwitch::new(&error_handling_pipeline))
                .add(BufferAllocator::new("encoded_frame_buffer", buffer_size))
                .add(TimestampAdder::new("encoding_start_timestamp"))
                .add(H264Encoder::new(buffer_size, width as i32, height as i32))
                .add(TimestampDiffCalculator::new(
                    "encoding_start_timestamp",
                    "encoding_time",
                ))
                .add(OnErrorSwitch::new(&error_handling_pipeline)),
        )
        .link(
            Component::new()
                .add(TimestampAdder::new("transmission_start_timestamp"))
                .add(SRTFrameSender::new(5001, Duration::from_millis(50)).await)
                .add(TimestampDiffCalculator::new(
                    "transmission_start_timestamp",
                    "transmission_time",
                ))
                .add(TimestampDiffCalculator::new(
                    "process_start_timestamp",
                    "total_time",
                ))
                .add(OnErrorSwitch::new(&error_handling_pipeline)),
        )
        .link(
            Component::new()
                .add(
                    ConsoleAverageStatsLogger::new()
                        .header("--- Computational times")
                        .log("encoded_size")
                        .log("encoding_time")
                        .log("transmission_time")
                        .log("total_time"),
                )
                .add(
                    ConsoleAverageStatsLogger::new()
                        .header("--- Delay times")
                        .log("capture_delay"),
                ),
        )
        .bind();

    let main_handle = main_pipeline.run();
    let error_handle = error_handling_pipeline.run();

    main_handle.await.unwrap();
    error_handle.await.unwrap();

    Ok(())
}
