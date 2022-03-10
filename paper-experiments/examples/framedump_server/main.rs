use std::{path::PathBuf, time::Duration};

use log::info;
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

    let args: Vec<String> = std::env::args().collect();
    let x264opts = &args[1];
    let srt_latency = *(&args[2].parse::<u64>().unwrap());

    info!("X264 options: {}", x264opts);
    info!("SRT Latency: {}", srt_latency);

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
                .add(rfb_pool.redeemer().soft())
                .add(ycb_pool.redeemer().soft())
                .add(crcb_pool.redeemer().soft())
                .add(cbcb_pool.redeemer().soft())
                .add(efb_pool.redeemer().soft())
                .add(
                    ConsoleDropReasonLogger::new()
                        .log(DropReason::StaleFrame)
                        .log(DropReason::ConnectionError)
                        .log(DropReason::CodecError)
                        .log(DropReason::NoAvailableBuffers),
                )
                .add(CSVFrameDataSerializer::new("server_drops.csv").log("capture_timestamp")),
        )
        .bind()
        .feedable();

    let frame_dump_pipeline = AscodePipeline::new()
        .link(
            Component::new()
                .add(TimestampAdder::new("dump_start_timestamp"))
                .add(RawFrameDumper::new(
                    "raw_frame_buffer",
                    PathBuf::from("/home/lorenzo/Scrivania/remotia-dumps/server_frames_dump/"),
                ))
                .add(TimestampDiffCalculator::new(
                    "dump_start_timestamp",
                    "dump_time",
                ))
                .add(
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
                .add(Ticker::new(250))
                .add(TimestampAdder::new("process_start_timestamp"))
                .add(TimestampAdder::new("capture_timestamp"))
                .add(rfb_pool.borrower())
                .add(OnErrorSwitch::new(&error_handling_pipeline))
                .add(capturer)
                .add(TimestampDiffCalculator::new(
                    "capture_timestamp",
                    "capture_time",
                ))
                .add(TimestampAdder::new(
                    "capturing_component_processing_finished",
                )),
        )
        .link(
            Component::new()
                .add(TimestampDiffCalculator::new(
                    "capturing_component_processing_finished",
                    "capturing_to_encoding_component_delay",
                ))
                .add(TimestampDiffCalculator::new(
                    "capture_timestamp",
                    "capture_delay",
                ))
                .add(ThresholdBasedFrameDropper::new("capture_delay", 15))
                .add(OnErrorSwitch::new(&error_handling_pipeline))
                .add(TimestampAdder::new(
                    "color_space_conversion_start_timestamp",
                ))
                .add(ycb_pool.borrower())
                .add(crcb_pool.borrower())
                .add(cbcb_pool.borrower())
                .add(OnErrorSwitch::new(&error_handling_pipeline))
                .add(RGBAToYUV420PConverter::new())
                .add(CloneSwitch::new(&frame_dump_pipeline))
                .add(rfb_pool.redeemer())
                .add(TimestampDiffCalculator::new(
                    "color_space_conversion_start_timestamp",
                    "color_space_conversion_time",
                ))
                .add(efb_pool.borrower())
                .add(OnErrorSwitch::new(&error_handling_pipeline))
                .add(TimestampAdder::new("encoding_start_timestamp"))
                .add(X264Encoder::new(
                    buffer_size,
                    width as i32,
                    height as i32,
                    &x264opts,
                ))
                .add(ycb_pool.redeemer())
                .add(crcb_pool.redeemer())
                .add(cbcb_pool.redeemer())
                .add(TimestampDiffCalculator::new(
                    "encoding_start_timestamp",
                    "encoding_time",
                ))
                .add(OnErrorSwitch::new(&error_handling_pipeline))
                .add(TimestampAdder::new(
                    "encoding_component_processing_finished",
                )),
        )
        .link(
            Component::new()
                .add(TimestampDiffCalculator::new(
                    "encoding_component_processing_finished",
                    "encoding_to_transmission_component_delay",
                ))
                .add(TimestampDiffCalculator::new(
                    "capture_timestamp",
                    "pre_transmission_delay",
                ))
                .add(ThresholdBasedFrameDropper::new(
                    "pre_transmission_delay",
                    200,
                ))
                .add(OnErrorSwitch::new(&error_handling_pipeline))
                .add(TimestampAdder::new("transmission_start_timestamp"))
                .add(SRTFrameSender::new(5001, Duration::from_millis(srt_latency)).await)
                .add(efb_pool.redeemer())
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
                        .log("capture_time")
                        .log("color_space_conversion_time")
                        .log("encoding_time")
                        .log("transmission_time")
                        .log("total_time"),
                )
                .add(
                    ConsoleAverageStatsLogger::new()
                        .header("--- Components communication delays")
                        .log("capturing_to_encoding_component_delay")
                        .log("encoding_to_transmission_component_delay"),
                )
                .add(
                    ConsoleAverageStatsLogger::new()
                        .header("--- Delay times")
                        .log("capture_delay")
                        .log("pre_transmission_delay"),
                )
                .add(
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
