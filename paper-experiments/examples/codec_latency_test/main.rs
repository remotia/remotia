use std::path::PathBuf;

use remotia::{
    error::DropReason,
    processors::{
        debug::random_dropper::RandomFrameDropper, error_switch::OnErrorSwitch,
        key_check::KeyChecker, ticker::Ticker,
    },
    server::pipeline::ascode::{component::Component, AscodePipeline},
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
                .add(KeyChecker::new("capture_timestamp"))
                .add(CSVFrameDataSerializer::new("drops.csv").log("capture_timestamp")),
        )
        .bind()
        .feedable();

    let main_pipeline = AscodePipeline::new()
        .tag("Main")
        .link(
            Component::new()
                .add(Ticker::new(500))
                .add(TimestampAdder::new("capture_timestamp"))
                .add(rfb_pool.borrower())
                .add(OnErrorSwitch::new(&error_handling_pipeline))
                .add(capturer),
        )
        .link(
            Component::new()
                .add(OnErrorSwitch::new(&error_handling_pipeline))
                .add(ycb_pool.borrower())
                .add(crcb_pool.borrower())
                .add(cbcb_pool.borrower())
                .add(OnErrorSwitch::new(&error_handling_pipeline))
                .add(RGBAToYUV420PConverter::new())
                .add(efb_pool.borrower())
                .add(OnErrorSwitch::new(&error_handling_pipeline))
                .add(X264Encoder::new(
                    buffer_size,
                    width as i32,
                    height as i32,
                    &x264opts,
                ))
                .add(RawFrameDumper::new(
                    "raw_frame_buffer",
                    PathBuf::from("./encoded_frames_dump/"),
                ))
                .add(rfb_pool.redeemer())
                .add(ycb_pool.redeemer())
                .add(crcb_pool.redeemer())
                .add(cbcb_pool.redeemer())
                .add(OnErrorSwitch::new(&error_handling_pipeline)),
        )
        .link(Component::new().add(RandomFrameDropper::new(0.5)))
        .link(
            Component::new()
                .add(rfb_pool.borrower())
                .add(OnErrorSwitch::new(&error_handling_pipeline))
                .add(H264Decoder::new())
                .add(efb_pool.redeemer())
                .add(RawFrameDumper::new(
                    "raw_frame_buffer",
                    PathBuf::from("./decoded_frames_dump/"),
                ))
                .add(rfb_pool.redeemer())
                .add(OnErrorSwitch::new(&error_handling_pipeline)),
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
