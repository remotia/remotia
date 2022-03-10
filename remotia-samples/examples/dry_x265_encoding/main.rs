use remotia::{
    error::DropReason,
    processors::{
        error_switch::OnErrorSwitch, frame_drop::threshold::ThresholdBasedFrameDropper,
        ticker::Ticker,
    },
    pipeline::ascode::{component::Component, AscodePipeline},
};
use remotia_buffer_utils::BufferAllocator;
use remotia_core_capturers::scrap::ScrapFrameCapturer;
use remotia_core_codecs::yuv420p::encoder::RGBAToYUV420PConverter;
use remotia_core_loggers::{errors::ConsoleDropReasonLogger, stats::ConsoleAverageStatsLogger};
use remotia_ffmpeg_codecs::encoders::x265::X265Encoder;
use remotia_profilation_utils::time::{add::TimestampAdder, diff::TimestampDiffCalculator};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let error_handling_pipeline = AscodePipeline::new()
        .tag("ErrorsHandler")
        .link(
            Component::new().append(
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

    let main_pipeline = AscodePipeline::new()
        .tag("ServerMain")
        .link(
            Component::new()
                .append(Ticker::new(30))
                .append(TimestampAdder::new("process_start_timestamp"))
                .append(BufferAllocator::new("raw_frame_buffer", buffer_size))
                .append(TimestampAdder::new("capture_timestamp"))
                .append(capturer),
        )
        .link(
            Component::new()
                .append(TimestampDiffCalculator::new(
                    "capture_timestamp",
                    "capture_delay",
                ))
                .append(ThresholdBasedFrameDropper::new("capture_delay", 10))
                .append(OnErrorSwitch::new(&error_handling_pipeline))
                .append(TimestampAdder::new(
                    "color_space_conversion_start_timestamp",
                ))
                .append(BufferAllocator::new("y_channel_buffer", width * height))
                .append(BufferAllocator::new(
                    "cb_channel_buffer",
                    width * height / 4,
                ))
                .append(BufferAllocator::new(
                    "cr_channel_buffer",
                    width * height / 4,
                ))
                .append(RGBAToYUV420PConverter::new())
                .append(TimestampDiffCalculator::new(
                    "color_space_conversion_start_timestamp",
                    "color_space_conversion_time",
                ))
                .append(BufferAllocator::new("encoded_frame_buffer", buffer_size))
                .append(TimestampAdder::new("encoding_start_timestamp"))
                .append(X265Encoder::new(
                    buffer_size,
                    width as i32,
                    height as i32,
                    "keyint=16",
                ))
                .append(TimestampDiffCalculator::new(
                    "encoding_start_timestamp",
                    "encoding_time",
                ))
                .append(OnErrorSwitch::new(&error_handling_pipeline)),
        )
        .link(
            Component::new()
                .append(
                    ConsoleAverageStatsLogger::new()
                        .header("--- Computational times")
                        .log("encoded_size")
                        .log("avframe_building_time")
                        .log("encoding_time"),
                )
                .append(
                    ConsoleAverageStatsLogger::new()
                        .header("--- Delay times")
                        .log("capture_delay"),
                ),
        )
        .bind();

    let mut handles = Vec::new();
    handles.extend(main_pipeline.run());
    handles.extend(error_handling_pipeline.run());

    for handle in handles {
        handle.await.unwrap()
    }

    Ok(())
}
