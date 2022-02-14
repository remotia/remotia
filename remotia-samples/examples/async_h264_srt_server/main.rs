use std::time::Duration;

use remotia::{
    error::DropReason,
    processors::{
        error_switch::OnErrorSwitch, frame_drop::threshold::ThresholdBasedFrameDropper,
        ticker::Ticker,
    },
    server::pipeline::ascode::{component::Component, AscodePipeline},
};
use remotia_buffer_utils::BufferAllocator;
use remotia_core_capturers::scrap::ScrapFrameCapturer;
use remotia_core_loggers::{errors::ConsoleDropReasonLogger, stats::ConsoleAverageStatsLogger};
use remotia_ffmpeg_codecs::encoders::asynchronous::encoders::h264::AsyncH264Encoder;
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
                    // .log(DropReason::StaleFrame)
                    // .log(DropReason::ConnectionError)
                    // .log(DropReason::CodecError),
            ),
        )
        .bind()
        .feedable();

    let capturer = ScrapFrameCapturer::new_from_primary();
    let width = capturer.width();
    let height = capturer.height();
    let buffer_size = width * height * 4;

    let encoder = AsyncH264Encoder::new(width as i32, height as i32);

    let frame_push_pipeline = AscodePipeline::new()
        .tag("FramePush")
        .link(
            Component::new()
                .add(Ticker::new(33))
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
                .add(TimestampAdder::new("push_start_timestamp"))
                .add(encoder.pusher())
                .add(OnErrorSwitch::new(&error_handling_pipeline))
                .add(TimestampDiffCalculator::new(
                    "push_start_timestamp",
                    "push_time",
                ))
                .add(
                    ConsoleAverageStatsLogger::new()
                        // .log("push_time")
                        // .log("push_mutex_lock_time")
                        // .log("send_frame_time")
                        .log("avframe_creation_time")
                ),
        )
        .bind();

    let frame_pull_pipeline = AscodePipeline::new()
        .tag("FramePull")
        .link(
            Component::new()
                .add(BufferAllocator::new("encoded_frame_buffer", buffer_size))
                .add(TimestampAdder::new("pull_start_timestamp"))
                .add(encoder.puller())
                .add(TimestampDiffCalculator::new(
                    "pull_start_timestamp",
                    "pull_time",
                ))
                .add(OnErrorSwitch::new(&error_handling_pipeline)),
        )
        /*.link(
            Component::new()
                /*.add(TimestampDiffCalculator::new(
                    "capture_timestamp",
                    "pre_transmission_delay",
                ))*/
                .add(ThresholdBasedFrameDropper::new("pre_transmission_delay", 50))
                .add(TimestampAdder::new("capture_timestamp")) // TODO: Remove
                .add(OnErrorSwitch::new(&error_handling_pipeline))
                .add(TimestampAdder::new("transmission_start_timestamp"))
                .add(SRTFrameSender::new(5001, Duration::from_millis(50)).await)
                .add(TimestampDiffCalculator::new(
                    "transmission_start_timestamp",
                    "transmission_time",
                ))
                /*.add(TimestampDiffCalculator::new(
                    "process_start_timestamp",
                    "total_time",
                ))*/
                .add(OnErrorSwitch::new(&error_handling_pipeline)),
        )*/
        .link(
            Component::new()
                .add(
                    ConsoleAverageStatsLogger::new()
                        // .header("--- Computational times")
                        // .log("encoded_size")
                        // .log("push_time")
                        // .log("pull_time")
                        // .log("pull_mutex_lock_time")
                        // .log("transmission_time")
                        // .log("total_time"),
                ), /*.add(
                       ConsoleAverageStatsLogger::new()
                           .header("--- Delay times")
                           .log("capture_delay"),
                   ),*/
        )
        .bind();

    let mut handles = Vec::new();
    handles.extend(frame_push_pipeline.run());
    handles.extend(frame_pull_pipeline.run());
    handles.extend(error_handling_pipeline.run());

    for handle in handles {
        handle.await.unwrap()
    }

    Ok(())
}
