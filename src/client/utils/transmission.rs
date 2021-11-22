use std::{ops::ControlFlow, time::Instant};

use log::{debug, error, warn};
use pixels::Pixels;

use crate::client::{decode::Decoder, error::ClientError, profiling::ReceivedFrameStats, receive::FrameReceiver, utils::decoding::packed_bgr_to_packed_rgba};

pub fn receive_frame(
    decoder: &mut Box<dyn Decoder>,
    pixels: &mut Pixels,
    frame_receiver: &mut Box<dyn FrameReceiver>,
    encoded_frame_buffer: &mut Vec<u8>,
    consecutive_connection_losses: &mut i32,
) -> ControlFlow<(), ReceivedFrameStats> {
    debug!("Waiting for next frame...");

    let total_start_time = Instant::now();

    let reception_start_time = Instant::now();
    let receive_result = frame_receiver.receive_encoded_frame(encoded_frame_buffer);
    let reception_time = reception_start_time.elapsed().as_millis();

    let decoding_start_time = Instant::now();
    let decode_result = receive_result.and_then(|received_data_length| {
        decode_task(decoder, &mut encoded_frame_buffer[..received_data_length])
    });
    let decoding_time = decoding_start_time.elapsed().as_millis();

    let rendering_start_time = Instant::now();
    let render_result =
        decode_result.and_then(|_| render_task(decoder, pixels, consecutive_connection_losses));
    let rendering_time = rendering_start_time.elapsed().as_millis();

    let rendered = render_result.is_ok();
    render_result.unwrap_or_else(|e| {
        handle_error(e, consecutive_connection_losses);
    });

    if *consecutive_connection_losses >= 100 {
        error!("Too much consecutive connection losses, closing stream");
        return ControlFlow::Break(());
    }

    let total_time = total_start_time.elapsed().as_millis();

    ControlFlow::Continue(ReceivedFrameStats {
        reception_time,
        decoding_time,
        rendering_time,
        total_time,
        rendered,
    })
}

fn handle_error(error: ClientError, consecutive_connection_losses: &mut i32) {
    match error {
        ClientError::InvalidWholeFrameHeader => *consecutive_connection_losses = 0,
        ClientError::FFMpegSendPacketError => {
            debug!("H264 Send packet error")
        }
        _ => *consecutive_connection_losses += 1,
    }

    warn!(
        "Error while receiving frame: {}, dropping (consecutive connection losses: {})",
        error, consecutive_connection_losses
    );
}

fn decode_task(
    decoder: &mut Box<dyn Decoder>,
    encoded_frame_buffer: &mut [u8],
) -> Result<usize, ClientError> {
    debug!("Decoding {} received bytes", encoded_frame_buffer.len());
    decoder.decode(encoded_frame_buffer)
}

fn render_task(
    decoder: &mut Box<dyn Decoder>,
    pixels: &mut Pixels,
    consecutive_connection_losses: &mut i32,
) -> Result<(), ClientError> {
    packed_bgr_to_packed_rgba(decoder.get_decoded_frame(), pixels.get_frame());

    *consecutive_connection_losses = 0;
    pixels.render().unwrap();
    debug!("[SUCCESS] Frame rendered on screen");

    Ok(())
}