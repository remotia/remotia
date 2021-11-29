use std::{io::Read, net::{TcpStream}};

use log::debug;

use crate::client::error::ClientError;

use super::FrameReceiver;

pub struct SRTFrameReceiver {
}

impl SRTFrameReceiver {
    pub fn new() -> Self {
        Self {
        }
    }

    fn receive_frame_header(&mut self) -> Result<usize, ClientError> {
        debug!("Receiving frame header...");
        Ok(0)
    }

    fn receive_frame_pixels(&mut self, frame_buffer: &mut[u8])  -> Result<usize, ClientError> {
        debug!("Receiving {} encoded frame bytes...", frame_buffer.len());

        let mut total_read_bytes = 0;

        Ok(total_read_bytes)
    }
}

impl FrameReceiver for SRTFrameReceiver {
    fn receive_encoded_frame(&mut self, frame_buffer: &mut[u8]) -> Result<usize, ClientError> {
        let frame_size = self.receive_frame_header()?;

        self.receive_frame_pixels(&mut frame_buffer[..frame_size])
    }
}