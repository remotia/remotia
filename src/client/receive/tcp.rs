use std::{io::Read, net::{TcpStream}};

use async_trait::async_trait;

use log::debug;

use crate::{client::{error::ClientError, receive::ReceivedFrame}, common::{feedback::FeedbackMessage, network::FrameBody}};

use super::FrameReceiver;

pub struct TCPFrameReceiver {
    stream: TcpStream,
}

impl TCPFrameReceiver {
    pub fn create(
        stream: TcpStream,
    ) -> Self {
        Self {
            stream,
        }
    }

    fn receive_frame_header(&mut self) -> Result<usize, ClientError> {
        debug!("Receiving frame header...");

        let mut frame_size_vec = [0 as u8; 8];

        let result = self.stream.read(&mut frame_size_vec);

        if result.is_err() {
            return Err(ClientError::InvalidWholeFrameHeader);
        }

        Ok(usize::from_be_bytes(frame_size_vec))
    }

    fn receive_frame_pixels(&mut self, binarized_obj_size: usize, encoded_frame_buffer: &mut[u8])  -> Result<ReceivedFrame, ClientError> {
        debug!("Receiving {} binarized bytes...", binarized_obj_size);

        let mut total_read_bytes = 0;

        let mut binarized_obj_buffer = vec![0 as u8; binarized_obj_size];

        while total_read_bytes < binarized_obj_size {
            let read_bytes = self.stream.read(&mut binarized_obj_buffer[total_read_bytes..]).unwrap();
            debug!("Received {} bytes", read_bytes); 

            if read_bytes == 0 {
                return Err(ClientError::EmptyFrame);
            }

            total_read_bytes += read_bytes;
        }

        debug!("Total bytes received: {}", total_read_bytes); 

        if total_read_bytes == 0 {
            return Err(ClientError::EmptyFrame);
        }

        let frame_body = bincode::deserialize::<FrameBody>(&binarized_obj_buffer).unwrap();
        let pixels_count = frame_body.frame_pixels.len();

        encoded_frame_buffer[..pixels_count].copy_from_slice(&frame_body.frame_pixels);

        Ok(ReceivedFrame {
            buffer_size: pixels_count,
            capture_timestamp: frame_body.capture_timestamp,
            reception_delay: 0
        })
    }
}

#[async_trait]
impl FrameReceiver for TCPFrameReceiver {
    async fn receive_encoded_frame(&mut self, encoded_frame_buffer: &mut[u8]) -> Result<ReceivedFrame, ClientError> {
        let binarized_obj_size = self.receive_frame_header()?;

        self.receive_frame_pixels(binarized_obj_size, encoded_frame_buffer)
    }

    fn handle_feedback(&mut self, message: FeedbackMessage) {
        debug!("Feedback message: {:?}", message);
    }
}