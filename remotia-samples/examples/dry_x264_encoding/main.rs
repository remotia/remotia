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
use remotia_core_codecs::yuv420p::encoder::RGBAToYUV420PConverter;
use remotia_core_loggers::{
    csv::serializer::CSVFrameDataSerializer, errors::ConsoleDropReasonLogger,
    stats::ConsoleAverageStatsLogger,
};
use remotia_ffmpeg_codecs::encoders::x264::X264Encoder;
use remotia_profilation_utils::time::{add::TimestampAdder, diff::TimestampDiffCalculator};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let error_handling_pipeline = AscodePipeline::new()
        .tag("ErrorsHandler")
        .link(
            Component::new()
                .add(
                    ConsoleDropReasonLogger::new()
                        .log(DropReason::StaleFrame)
                        .log(DropReason::ConnectionError)
                        .log(DropReason::CodecError),
                )
                .add(
                    ConsoleAverageStatsLogger::new()
                        .header("--- Dropped frames delay times")
                        .log("capture_delay"),
                ),
        )
        .bind()
        .feedable();

    let capturer = ScrapFrameCapturer::new_from_primary();
    let width = capturer.width();
    let height = capturer.height();
    let buffer_size = width * height * 4;

    let encoding_pipeline = AscodePipeline::new()
        .tag("Encoding")
        .link(
            Component::new()
                .add(TimestampAdder::new("encoding_start_timestamp"))
                .add(BufferAllocator::new("encoded_frame_buffer", buffer_size))
                .add(X264Encoder::new(
                    buffer_size,
                    width as i32,
                    height as i32,
                    "keyint=16",
                ))
                .add(TimestampDiffCalculator::new(
                    "encoding_start_timestamp",
                    "encoding_time",
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
                        .log("capture_time")
                        .log("color_space_conversion_time")
                        .log("encoding_time")
                        .log("total_time"),
                )
                .add(
                    ConsoleAverageStatsLogger::new()
                        .header("--- Delay times")
                        .log("capture_delay"),
                )
                .add(
                    CSVFrameDataSerializer::new("dry_h264_encoding_logs.csv")
                        .log("capture_timestamp")
                        .log("capture_time")
                        .log("encoding_time")
                        .log("encoded_size"),
                ),
        )
        .bind()
        .feedable();

    let converters = 2;
    let mut conversion_switch = PoolingSwitch::new();
    let conversion_pipelines: Vec<AscodePipeline> = (0..converters)
        .map(|_| {
            build_color_conversion_pipeline(
                width,
                height,
                &encoding_pipeline,
                &error_handling_pipeline,
            )
        })
        .collect();

    for key in 0..converters {
        conversion_switch =
            conversion_switch.entry(key, conversion_pipelines.get(key as usize).unwrap());
    }

    let capturing_pipeline = AscodePipeline::new()
        .tag("Capturing")
        .link(
            Component::new()
                .add(Ticker::new(20))
                .add(TimestampAdder::new("process_start_timestamp"))
                .add(BufferAllocator::new("raw_frame_buffer", buffer_size))
                .add(TimestampAdder::new("capture_timestamp"))
                .add(capturer)
                .add(TimestampDiffCalculator::new(
                    "capture_timestamp",
                    "capture_time",
                ))
                .add(conversion_switch)
                // .add(Switch::new(&encoding_pipeline)),
        )
        .bind();

    let mut handles = Vec::new();
    handles.extend(capturing_pipeline.run());
    handles.extend(encoding_pipeline.run());
    for conversion_pipeline in conversion_pipelines {
        handles.extend(conversion_pipeline.run());
    }
    handles.extend(error_handling_pipeline.run());

    for handle in handles {
        handle.await.unwrap()
    }

    Ok(())
}

fn build_color_conversion_pipeline(
    width: usize,
    height: usize,
    encoding_pipeline: &AscodePipeline,
    error_handling_pipeline: &AscodePipeline,
) -> AscodePipeline {
    AscodePipeline::new()
        .tag("ColorSpaceConversion")
        .link(
            Component::new()
                .add(TimestampDiffCalculator::new(
                    "capture_timestamp",
                    "capture_delay",
                ))
                .add(ThresholdBasedFrameDropper::new("capture_delay", 10))
                .add(OnErrorSwitch::new(&error_handling_pipeline))
                .add(TimestampAdder::new(
                    "color_space_conversion_start_timestamp",
                ))
                .add(BufferAllocator::new("y_channel_buffer", width * height))
                .add(BufferAllocator::new(
                    "cb_channel_buffer",
                    width * height / 4,
                ))
                .add(BufferAllocator::new(
                    "cr_channel_buffer",
                    width * height / 4,
                ))
                .add(RGBAToYUV420PConverter::new())
                .add(TimestampDiffCalculator::new(
                    "color_space_conversion_start_timestamp",
                    "color_space_conversion_time",
                ))
                .add(Switch::new(&encoding_pipeline)),
        )
        .bind()
        .feedable()
}
