use std::{path::PathBuf, time::Duration};

use remotia::{
    error::DropReason,
    processors::{
        clone_switch::CloneSwitch, error_switch::OnErrorSwitch,
        frame_drop::threshold::ThresholdBasedFrameDropper, ticker::Ticker,
    },
    pipeline::ascode::{component::Component, AscodePipeline},
};
use remotia_buffer_utils::pool::BuffersPool;
use remotia_core_capturers::scrap::ScrapFrameCapturer;
use remotia_core_codecs::yuv420p::encoder::RGBAToYUV420PConverter;
use remotia_core_loggers::{
    csv::serializer::CSVFrameDataSerializer, errors::ConsoleDropReasonLogger,
    frame_dump::RawFrameDumper, stats::ConsoleAverageStatsLogger,
};
use remotia_ffmpeg_codecs::encoders::x264::X264Encoder;
use remotia_profilation_utils::time::{add::TimestampAdder, diff::TimestampDiffCalculator};
use remotia_srt::sender::SRTFrameSender;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let capturer = ScrapFrameCapturer::new_from_primary();
    let width = capturer.width();
    let height = capturer.height();
    let buffer_size = width * height * 4;

    let rfb_pool = BuffersPool::new("raw_frame_buffer", 8, buffer_size);
    let ycb_pool = BuffersPool::new("y_channel_buffer", 8, width * height);
    let crcb_pool = BuffersPool::new("cr_channel_buffer", 8, (width * height) / 4);
    let cbcb_pool = BuffersPool::new("cb_channel_buffer", 8, (width * height) / 4);
    let efb_pool = BuffersPool::new("encoded_frame_buffer", 8, buffer_size);

    let error_handling_pipeline = AscodePipeline::new()
        .tag("ErrorsHandler")
        .link(
            Component::new()
                .append(rfb_pool.redeemer().soft())
                .append(ycb_pool.redeemer().soft())
                .append(crcb_pool.redeemer().soft())
                .append(cbcb_pool.redeemer().soft())
                .append(efb_pool.redeemer().soft())
                .append(
                    ConsoleDropReasonLogger::new()
                        .log(DropReason::StaleFrame)
                        .log(DropReason::ConnectionError)
                        .log(DropReason::CodecError)
                        .log(DropReason::NoAvailableBuffers),
                )
                .append(CSVFrameDataSerializer::new("server_drops.csv").log("capture_timestamp")),
        )
        .bind()
        .feedable();

    let frame_dump_pipeline = AscodePipeline::new()
        .link(
            Component::new()
                .append(TimestampAdder::new("dump_start_timestamp"))
                .append(RawFrameDumper::new(
                    "raw_frame_buffer",
                    PathBuf::from("/home/lorenzo/Scrivania/remotia-dumps/server_frames_dump/"),
                ))
                .append(TimestampDiffCalculator::new(
                    "dump_start_timestamp",
                    "dump_time",
                ))
                .append(
                    ConsoleAverageStatsLogger::new()
                        .header("--- Frame dump times")
                        .log("dump_time"),
                ),
        )
        .bind()
        .feedable();

    let main_pipeline = AscodePipeline::new()
        .tag("ServerMain")
        .link(
            Component::new()
                .append(Ticker::new(50))
                .append(TimestampAdder::new("process_start_timestamp"))
                .append(TimestampAdder::new("capture_timestamp"))
                .append(rfb_pool.borrower())
                .append(OnErrorSwitch::new(&error_handling_pipeline))
                .append(capturer)
                .append(TimestampDiffCalculator::new(
                    "capture_timestamp",
                    "capture_time",
                ))
                .append(TimestampAdder::new(
                    "capturing_component_processing_finished",
                )),
        )
        .link(
            Component::new()
                .append(TimestampDiffCalculator::new(
                    "capturing_component_processing_finished",
                    "capturing_to_encoding_component_delay",
                ))
                .append(TimestampDiffCalculator::new(
                    "capture_timestamp",
                    "capture_delay",
                ))
                .append(ThresholdBasedFrameDropper::new("capture_delay", 15))
                .append(OnErrorSwitch::new(&error_handling_pipeline))
                .append(TimestampAdder::new(
                    "color_space_conversion_start_timestamp",
                ))
                .append(ycb_pool.borrower())
                .append(crcb_pool.borrower())
                .append(cbcb_pool.borrower())
                .append(OnErrorSwitch::new(&error_handling_pipeline))
                .append(RGBAToYUV420PConverter::new())
                .append(CloneSwitch::new(&frame_dump_pipeline))
                .append(rfb_pool.redeemer())
                .append(TimestampDiffCalculator::new(
                    "color_space_conversion_start_timestamp",
                    "color_space_conversion_time",
                ))
                .append(efb_pool.borrower())
                .append(OnErrorSwitch::new(&error_handling_pipeline))
                .append(TimestampAdder::new("encoding_start_timestamp"))
                .append(X264Encoder::new(
                    buffer_size,
                    width as i32,
                    height as i32,
                    "keyint=16",
                ))
                // .add(LibVpxVP9Encoder::new(buffer_size, width as i32, height as i32))
                .append(ycb_pool.redeemer())
                .append(crcb_pool.redeemer())
                .append(cbcb_pool.redeemer())
                .append(TimestampDiffCalculator::new(
                    "encoding_start_timestamp",
                    "encoding_time",
                ))
                .append(OnErrorSwitch::new(&error_handling_pipeline))
                .append(TimestampAdder::new(
                    "encoding_component_processing_finished",
                )),
        )
        .link(
            Component::new()
                .append(TimestampDiffCalculator::new(
                    "encoding_component_processing_finished",
                    "encoding_to_transmission_component_delay",
                ))
                .append(TimestampDiffCalculator::new(
                    "capture_timestamp",
                    "pre_transmission_delay",
                ))
                .append(ThresholdBasedFrameDropper::new(
                    "pre_transmission_delay",
                    200,
                ))
                .append(OnErrorSwitch::new(&error_handling_pipeline))
                .append(TimestampAdder::new("transmission_start_timestamp"))
                .append(SRTFrameSender::new(5001, Duration::from_millis(50)).await)
                .append(efb_pool.redeemer())
                .append(TimestampDiffCalculator::new(
                    "transmission_start_timestamp",
                    "transmission_time",
                ))
                .append(TimestampDiffCalculator::new(
                    "process_start_timestamp",
                    "total_time",
                ))
                .append(OnErrorSwitch::new(&error_handling_pipeline)),
        )
        .link(
            Component::new()
                .append(
                    ConsoleAverageStatsLogger::new()
                        .header("--- Computational times")
                        .log("encoded_size")
                        .log("capture_time")
                        .log("color_space_conversion_time")
                        .log("encoding_time")
                        .log("transmission_time")
                        .log("total_time"),
                )
                .append(
                    ConsoleAverageStatsLogger::new()
                        .header("--- Components communication delays")
                        .log("capturing_to_encoding_component_delay")
                        .log("encoding_to_transmission_component_delay"),
                )
                .append(
                    ConsoleAverageStatsLogger::new()
                        .header("--- Delay times")
                        .log("capture_delay")
                        .log("pre_transmission_delay"),
                )
                .append(
                    CSVFrameDataSerializer::new("server.csv")
                        .log("capture_timestamp")
                        .log("encoded_size")
                        .log("capture_time")
                        .log("color_space_conversion_time")
                        .log("encoding_time")
                        .log("transmission_time")
                        .log("total_time")
                        .log("capture_delay")
                        .log("pre_transmission_delay"),
                ),
        )
        .bind();

    let mut handles = Vec::new();
    handles.extend(main_pipeline.run());
    handles.extend(error_handling_pipeline.run());
    handles.extend(frame_dump_pipeline.run());

    for handle in handles {
        handle.await.unwrap()
    }

    Ok(())
}
