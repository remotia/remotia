use std::path::PathBuf;

use remotia::{
    error::DropReason,
    processors::{
        debug::random_dropper::RandomFrameDropper, error_switch::OnErrorSwitch,
        key_check::KeyChecker, ticker::Ticker,
    },
    pipeline::ascode::{component::Component, AscodePipeline},
};
use remotia_buffer_utils::pool::BuffersPool;
use remotia_core_capturers::scrap::ScrapFrameCapturer;
use remotia_core_codecs::yuv420p::encoder::RGBAToYUV420PConverter;
use remotia_core_loggers::{
    csv::serializer::CSVFrameDataSerializer, errors::ConsoleDropReasonLogger,
    frame_dump::RawFrameDumper,
};
use remotia_ffmpeg_codecs::{decoders::h264::H264Decoder, encoders::x264::X264Encoder};
use remotia_profilation_utils::time::add::TimestampAdder;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let x264opts = "keyint=16";

    let capturer = ScrapFrameCapturer::new_from_primary();
    let width = capturer.width();
    let height = capturer.height();
    let buffer_size = width * height * 4;

    let rfb_pool = BuffersPool::new("raw_frame_buffer", 128, buffer_size);
    let ycb_pool = BuffersPool::new("y_channel_buffer", 128, width * height);
    let crcb_pool = BuffersPool::new("cr_channel_buffer", 128, (width * height) / 4);
    let cbcb_pool = BuffersPool::new("cb_channel_buffer", 128, (width * height) / 4);
    let efb_pool = BuffersPool::new("encoded_frame_buffer", 128, buffer_size);

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
                .append(KeyChecker::new("capture_timestamp"))
                .append(CSVFrameDataSerializer::new("drops.csv").log("capture_timestamp")),
        )
        .bind()
        .feedable();

    let main_pipeline = AscodePipeline::new()
        .tag("Main")
        .link(
            Component::new()
                .append(Ticker::new(500))
                .append(TimestampAdder::new("capture_timestamp"))
                .append(rfb_pool.borrower())
                .append(OnErrorSwitch::new(&error_handling_pipeline))
                .append(capturer),
        )
        .link(
            Component::new()
                .append(OnErrorSwitch::new(&error_handling_pipeline))
                .append(ycb_pool.borrower())
                .append(crcb_pool.borrower())
                .append(cbcb_pool.borrower())
                .append(OnErrorSwitch::new(&error_handling_pipeline))
                .append(RGBAToYUV420PConverter::new())
                .append(efb_pool.borrower())
                .append(OnErrorSwitch::new(&error_handling_pipeline))
                .append(X264Encoder::new(
                    buffer_size,
                    width as i32,
                    height as i32,
                    &x264opts,
                ))
                .append(RawFrameDumper::new(
                    "raw_frame_buffer",
                    PathBuf::from("./encoded_frames_dump/"),
                ))
                .append(rfb_pool.redeemer())
                .append(ycb_pool.redeemer())
                .append(crcb_pool.redeemer())
                .append(cbcb_pool.redeemer())
                .append(OnErrorSwitch::new(&error_handling_pipeline)),
        )
        .link(Component::new().append(RandomFrameDropper::new(0.5)))
        .link(
            Component::new()
                .append(rfb_pool.borrower())
                .append(OnErrorSwitch::new(&error_handling_pipeline))
                .append(H264Decoder::new())
                .append(efb_pool.redeemer())
                .append(RawFrameDumper::new(
                    "raw_frame_buffer",
                    PathBuf::from("./decoded_frames_dump/"),
                ))
                .append(rfb_pool.redeemer())
                .append(OnErrorSwitch::new(&error_handling_pipeline)),
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
