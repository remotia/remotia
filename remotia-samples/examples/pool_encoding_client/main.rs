use std::time::Duration;

use remotia::{
    error::DropReason,
    processors::{
        error_switch::OnErrorSwitch,
        frame_drop::{
            threshold::ThresholdBasedFrameDropper, timestamp::TimestampBasedFrameDropper,
        },
        pool_switch::DepoolingSwitch,
        switch::Switch,
    },
    pipeline::ascode::{component::Component, AscodePipeline},
};
use remotia_buffer_utils::BufferAllocator;
use remotia_core_loggers::{
    errors::ConsoleDropReasonLogger, printer::ConsoleFrameDataPrinter,
    stats::ConsoleAverageStatsLogger,
};
use remotia_core_renderers::beryllium::BerylliumRenderer;
use remotia_ffmpeg_codecs::decoders::h264::H264Decoder;
use remotia_profilation_utils::time::{add::TimestampAdder, diff::TimestampDiffCalculator};
use remotia_srt::receiver::SRTFrameReceiver;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let error_handling_pipeline = build_errors_handling_pipeline();

    let width = 1280;
    let height = 720;
    let buffer_size = width * height * 4;

    let tail_pipeline = build_tail_pipeline(&error_handling_pipeline, width, height);

    let decoders_count = 4;
    let mut decoding_switch = DepoolingSwitch::new();

    let decoding_pipelines: Vec<AscodePipeline> = (0..decoders_count)
        .map(|_| build_decoding_pipeline(buffer_size, &error_handling_pipeline, &tail_pipeline))
        .collect();

    for key in 0..decoders_count {
        decoding_switch = decoding_switch.entry(key, decoding_pipelines.get(key as usize).unwrap());
    }

    let reception_pipeline = AscodePipeline::new()
        .tag("Reception")
        .link(
            Component::new()
                .append(BufferAllocator::new("encoded_frame_buffer", buffer_size))
                .append(TimestampAdder::new("reception_start_timestamp"))
                .append(SRTFrameReceiver::new("127.0.0.1:5001", Duration::from_millis(50)).await)
                .append(TimestampDiffCalculator::new(
                    "reception_start_timestamp",
                    "reception_time",
                ))
                .append(OnErrorSwitch::new(&error_handling_pipeline))
                .append(decoding_switch),
        )
        .bind();

    let mut handles = Vec::new();
    handles.extend(error_handling_pipeline.run());

    handles.extend(reception_pipeline.run());
    for decoding_pipeline in decoding_pipelines {
        handles.extend(decoding_pipeline.run());
    }
    handles.extend(tail_pipeline.run());

    for handle in handles {
        handle.await.unwrap()
    }

    Ok(())
}

fn build_tail_pipeline(
    error_handling_pipeline: &AscodePipeline,
    width: usize,
    height: usize,
) -> AscodePipeline {
    AscodePipeline::new()
        .tag("RenderingAndProfilation")
        .link(
            Component::new()
                .append(TimestampBasedFrameDropper::new("capture_timestamp"))
                /*.add(TimestampBasedFrameReorderingBuffer::new(
                    "capture_timestamp",
                    20,
                ))*/
                .append(OnErrorSwitch::new(error_handling_pipeline))
                .append(TimestampDiffCalculator::new(
                    "capture_timestamp",
                    "pre_render_frame_delay",
                ))
                .append(ThresholdBasedFrameDropper::new(
                    "pre_render_frame_delay",
                    200,
                ))
                .append(OnErrorSwitch::new(error_handling_pipeline))
                .append(TimestampAdder::new("rendering_start_timestamp"))
                .append(BerylliumRenderer::new(width as u32, height as u32))
                .append(TimestampDiffCalculator::new(
                    "rendering_start_timestamp",
                    "rendering_time",
                ))
                .append(TimestampDiffCalculator::new(
                    "reception_start_timestamp",
                    "total_time",
                ))
                .append(TimestampDiffCalculator::new(
                    "capture_timestamp",
                    "frame_delay",
                ))
                .append(OnErrorSwitch::new(error_handling_pipeline))
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
                ),
        )
        .bind()
        .feedable()
}

fn build_decoding_pipeline(
    buffer_size: usize,
    error_handling_pipeline: &AscodePipeline,
    tail_pipeline: &AscodePipeline,
) -> AscodePipeline {
    AscodePipeline::new()
        .tag("Decoding")
        .link(
            Component::new()
                .append(BufferAllocator::new("raw_frame_buffer", buffer_size))
                .append(TimestampAdder::new("decoding_start_timestamp"))
                .append(H264Decoder::new())
                .append(TimestampDiffCalculator::new(
                    "decoding_start_timestamp",
                    "decoding_time",
                ))
                .append(OnErrorSwitch::new(error_handling_pipeline))
                .append(Switch::new(tail_pipeline)),
        )
        .bind()
        .feedable()
}

fn build_errors_handling_pipeline() -> AscodePipeline {
    AscodePipeline::new()
        .tag("ErrorsHandler")
        .link(
            Component::new()
                .append(
                    ConsoleDropReasonLogger::new()
                        .log(DropReason::StaleFrame)
                        .log(DropReason::ConnectionError)
                        .log(DropReason::CodecError),
                )
                .append(ConsoleFrameDataPrinter::new()),
        )
        .bind()
        .feedable()
}
