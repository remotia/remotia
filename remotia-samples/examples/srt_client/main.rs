use std::{path::PathBuf, time::Duration};

use remotia::{
    error::DropReason,
    processors::{
        error_switch::OnErrorSwitch, frame_drop::threshold::ThresholdBasedFrameDropper,
        key_check::KeyChecker, ticker::Ticker, clone_switch::CloneSwitch,
    },
    pipeline::ascode::{component::Component, AscodePipeline},
};
use remotia_buffer_utils::pool::BuffersPool;
use remotia_core_loggers::{
    csv::serializer::CSVFrameDataSerializer, errors::ConsoleDropReasonLogger,
    frame_dump::RawFrameDumper, stats::ConsoleAverageStatsLogger,
};
use remotia_core_renderers::beryllium::BerylliumRenderer;
use remotia_ffmpeg_codecs::decoders::h264::H264Decoder;
use remotia_profilation_utils::time::{add::TimestampAdder, diff::TimestampDiffCalculator};
use remotia_srt::receiver::SRTFrameReceiver;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let width = 1280;
    let height = 720;
    let buffer_size = width * height * 4;

    let efb_pool = BuffersPool::new("encoded_frame_buffer", 8, buffer_size);
    let rfb_pool = BuffersPool::new("raw_frame_buffer", 8, buffer_size);

    let error_handling_pipeline = AscodePipeline::new()
        .tag("ErrorsHandler")
        .link(
            Component::new()
                .append(rfb_pool.redeemer().soft())
                .append(efb_pool.redeemer().soft())
                .append(
                    ConsoleDropReasonLogger::new()
                        .log(DropReason::StaleFrame)
                        .log(DropReason::ConnectionError)
                        .log(DropReason::CodecError)
                        .log(DropReason::NoDecodedFrames)
                        .log(DropReason::ConnectionError)
                        .log(DropReason::NoAvailableBuffers),
                )
                .append(KeyChecker::new("capture_timestamp"))
                .append(CSVFrameDataSerializer::new("client_drops.csv").log("capture_timestamp")),
        )
        .bind()
        .feedable();

    let frame_dump_pipeline = AscodePipeline::new()
        .link(
            Component::new()
                .append(TimestampAdder::new("dump_start_timestamp"))
                .append(RawFrameDumper::new(
                    "raw_frame_buffer",
                    PathBuf::from("/home/lorenzo/Scrivania/remotia-dumps/client_frames_dump/"),
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

    // Pipeline structure
    let main_pipeline = AscodePipeline::new()
        .tag("ClientMain")
        .link(
            Component::new()
                .append(Ticker::new(10))
                .append(efb_pool.borrower())
                .append(OnErrorSwitch::new(&error_handling_pipeline))
                .append(TimestampAdder::new("reception_start_timestamp"))
                .append(SRTFrameReceiver::new("127.0.0.1:5001", Duration::from_millis(50)).await)
                .append(TimestampDiffCalculator::new(
                    "reception_start_timestamp",
                    "reception_time",
                ))
                .append(OnErrorSwitch::new(&error_handling_pipeline)),
        )
        .link(
            Component::new()
                .append(rfb_pool.borrower())
                .append(OnErrorSwitch::new(&error_handling_pipeline))
                .append(TimestampAdder::new("decoding_start_timestamp"))
                .append(H264Decoder::new())
                // .add(H265Decoder::new())
                // .add(LibVpxVP9Decoder::new())
                .append(TimestampDiffCalculator::new(
                    "decoding_start_timestamp",
                    "decoding_time",
                ))
                .append(efb_pool.redeemer())
                .append(OnErrorSwitch::new(&error_handling_pipeline)),
        )
        .link(
            Component::new()
                .append(TimestampDiffCalculator::new(
                    "capture_timestamp",
                    "pre_render_frame_delay",
                ))
                .append(ThresholdBasedFrameDropper::new(
                    "pre_render_frame_delay",
                    200,
                ))
                .append(OnErrorSwitch::new(&error_handling_pipeline))
                .append(TimestampAdder::new("rendering_start_timestamp"))
                .append(BerylliumRenderer::new(width as u32, height as u32))
                .append(TimestampDiffCalculator::new(
                    "rendering_start_timestamp",
                    "rendering_time",
                ))
                .append(CloneSwitch::new(&frame_dump_pipeline))
                .append(rfb_pool.redeemer())
                .append(OnErrorSwitch::new(&error_handling_pipeline))
                .append(TimestampDiffCalculator::new(
                    "reception_start_timestamp",
                    "total_time",
                ))
                .append(TimestampDiffCalculator::new(
                    "capture_timestamp",
                    "frame_delay",
                ))
                .append(OnErrorSwitch::new(&error_handling_pipeline)),
        )
        .link(
            Component::new()
                .append(
                    ConsoleAverageStatsLogger::new()
                        .header("--- Computational times")
                        .log("reception_time")
                        .log("decoding_time")
                        .log("rendering_time")
                        .log("total_time"),
                )
                .append(
                    ConsoleAverageStatsLogger::new()
                        .header("--- Delay times")
                        .log("reception_delay")
                        .log("frame_delay"),
                )
                .append(
                    CSVFrameDataSerializer::new("client.csv")
                        .log("capture_timestamp")
                        .log("reception_time")
                        .log("decoding_time")
                        .log("rendering_time")
                        .log("total_time")
                        .log("reception_delay")
                        .log("frame_delay"),
                ),
        )
        .bind();

    let mut handles = Vec::new();
    handles.extend(main_pipeline.run());
    handles.extend(frame_dump_pipeline.run());
    handles.extend(error_handling_pipeline.run());

    for handle in handles {
        handle.await.unwrap()
    }

    Ok(())
}
