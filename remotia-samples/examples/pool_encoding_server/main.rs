use std::time::Duration;

use remotia::{
    error::DropReason,
    processors::{
        error_switch::OnErrorSwitch, frame_drop::threshold::ThresholdBasedFrameDropper,
        pool_switch::PoolingSwitch, switch::Switch, ticker::Ticker,
    },
    server::pipeline::ascode::{component::Component, AscodePipeline},
};
use remotia_buffer_utils::BufferAllocator;
use remotia_core_capturers::scrap::ScrapFrameCapturer;
use remotia_core_loggers::{errors::ConsoleDropReasonLogger, stats::ConsoleAverageStatsLogger};
use remotia_ffmpeg_codecs::encoders::x264::X264Encoder;
use remotia_profilation_utils::time::{add::TimestampAdder, diff::TimestampDiffCalculator};
use remotia_srt::sender::SRTFrameSender;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let error_handling_pipeline = AscodePipeline::new()
        .tag("ErrorsHandler")
        .link(
            Component::new().add(
                ConsoleDropReasonLogger::new()
                    .log(DropReason::StaleFrame)
                    .log(DropReason::ConnectionError)
                    .log(DropReason::CodecError),
            ),
        )
        .bind()
        .feedable();

    let capturer = ScrapFrameCapturer::new_from_primary();
    let width = capturer.width();
    let height = capturer.height();
    let buffer_size = width * height * 4;

    let tail_pipeline = build_tail_pipeline(&error_handling_pipeline).await;

    let encoders_count = 4;
    let mut encoding_switch = PoolingSwitch::new();
    let encoding_pipelines: Vec<AscodePipeline> = (0..encoders_count)
        .map(|_| {
            build_encoding_pipeline(
                &error_handling_pipeline,
                &tail_pipeline,
                buffer_size,
                width,
                height,
            )
        })
        .collect();

    for key in 0..encoders_count {
        encoding_switch = encoding_switch.entry(key, encoding_pipelines.get(key as usize).unwrap());
    }

    let capturing_pipeline = AscodePipeline::new()
        .tag("Capturing")
        .link(
            Component::new()
                .add(Ticker::new(50))
                .add(TimestampAdder::new("process_start_timestamp"))
                .add(BufferAllocator::new("raw_frame_buffer", buffer_size))
                .add(TimestampAdder::new("capture_timestamp"))
                .add(capturer)
                .add(encoding_switch),
        )
        .bind();

    let mut handles = Vec::new();
    handles.extend(error_handling_pipeline.run());

    handles.extend(capturing_pipeline.run());
    for encoding_pipeline in encoding_pipelines {
        handles.extend(encoding_pipeline.run());
    }
    handles.extend(tail_pipeline.run());

    for handle in handles {
        handle.await.unwrap()
    }

    Ok(())
}

async fn build_tail_pipeline(error_handling_pipeline: &AscodePipeline) -> AscodePipeline {
    AscodePipeline::new()
        .tag("TransmissionAndProfilation")
        .link(
            Component::new()
                .add(TimestampDiffCalculator::new(
                    "capture_timestamp",
                    "pre_transmission_delay",
                ))
                .add(ThresholdBasedFrameDropper::new(
                    "pre_transmission_delay",
                    50,
                ))
                .add(OnErrorSwitch::new(error_handling_pipeline))
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
                .add(OnErrorSwitch::new(error_handling_pipeline))
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
        .bind()
        .feedable()
}

fn build_encoding_pipeline(
    error_handling_pipeline: &AscodePipeline,
    tail_pipeline: &AscodePipeline,
    buffer_size: usize,
    width: usize,
    height: usize,
) -> AscodePipeline {
    AscodePipeline::new()
        .tag("Encoding")
        .link(
            Component::new()
                .add(TimestampDiffCalculator::new(
                    "capture_timestamp",
                    "capture_delay",
                ))
                .add(ThresholdBasedFrameDropper::new("capture_delay", 10))
                .add(OnErrorSwitch::new(error_handling_pipeline))
                .add(BufferAllocator::new("encoded_frame_buffer", buffer_size))
                .add(TimestampAdder::new("encoding_start_timestamp"))
                .add(X264Encoder::new(buffer_size, width as i32, height as i32, ""))
                .add(TimestampDiffCalculator::new(
                    "encoding_start_timestamp",
                    "encoding_time",
                ))
                .add(OnErrorSwitch::new(error_handling_pipeline))
                .add(Switch::new(&tail_pipeline)),
        )
        .bind()
        .feedable()
}
