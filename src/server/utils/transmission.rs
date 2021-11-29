use std::time::Instant;

use log::debug;
use scrap::{Capturer, Frame};

use crate::server::{capture, encode::Encoder, profiling::TransmittedFrameStats, send::FrameSender, utils::encoding::packed_bgra_to_packed_bgr};


pub async fn transmit_frame(
    capturer: &mut Capturer,
    packed_bgr_frame_buffer: &mut [u8],
    encoder: &mut dyn Encoder,
    frame_sender: &mut dyn FrameSender,
) -> Result<TransmittedFrameStats, std::io::Error> {
    let loop_start_time = Instant::now();

    // Capture frame
    let result = capture::capture_frame(capturer);

    debug!("Frame captured");

    let packed_bgra_frame_buffer: Frame = match result {
        Ok(buffer) => buffer,
        Err(error) => {
            return Err(error);
        }
    };

    debug!("Encoding...");

    let encoding_start_time = Instant::now();

    debug!(
        "{} {}",
        packed_bgra_frame_buffer.len(),
        packed_bgr_frame_buffer.len()
    );

    packed_bgra_to_packed_bgr(&packed_bgra_frame_buffer, packed_bgr_frame_buffer);
    let encoded_size = encoder.encode(&packed_bgr_frame_buffer);

    let encoding_time = encoding_start_time.elapsed().as_millis();

    debug!("Encoding time: {}", encoding_time);

    debug!(
        "Encoded frame size: {}/{}",
        encoded_size,
        packed_bgra_frame_buffer.len()
    );

    let transfer_start_time = Instant::now();

    debug!(
        "Encoded frame slice length: {}",
        encoder.get_encoded_frame().len()
    );

    frame_sender.send_frame(encoder.get_encoded_frame()).await;

    let transfer_time = transfer_start_time.elapsed().as_millis();
    debug!("Transfer time: {}", transfer_time);

    let total_time = loop_start_time.elapsed().as_millis();
    debug!("Total time: {}", total_time);

    Ok(TransmittedFrameStats {
        encoding_time,
        transfer_time,
        total_time,
        encoded_size,
    })
}
