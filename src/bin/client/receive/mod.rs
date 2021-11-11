use crate::error::ClientError;

pub mod udp;

pub trait FrameReceiver {
    fn receive_frame(&self, frame_buffer: & mut[u8]) -> Result<(), ClientError>;
}