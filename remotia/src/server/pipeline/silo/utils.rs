use bytes::BytesMut;
use log::debug;
use tokio::sync::mpsc::UnboundedSender;

use crate::types::FrameData;

pub fn return_writable_buffer(
    raw_frame_buffers_sender: &UnboundedSender<BytesMut>,
    frame_data: &mut FrameData,
    buffer_id: &str,
) {
    if frame_data.has_writable_buffer(buffer_id) {
        debug!("Returning empty '{}'...", buffer_id);
        raw_frame_buffers_sender
            .send(frame_data.extract_writable_buffer(buffer_id).unwrap())
            .expect("Buffer return error");
    } else {
        debug!("Attempt to return a missing '{}'...", buffer_id);
    }
}