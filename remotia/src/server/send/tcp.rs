use std::{io::Write, net::TcpStream};

use async_trait::async_trait;
use bytes::Bytes;
use log::debug;

use crate::{common::{feedback::FeedbackMessage, network::FrameBody}, types::FrameData};

use super::FrameSender;

pub struct TCPFrameSender {
    stream: TcpStream
}

impl TCPFrameSender {
    pub fn new(
        stream: TcpStream
    ) -> Self {
        Self {
            stream
        }
    }

    fn send_packet_header(&mut self, frame_size: usize) {
        self.stream.write_all(&frame_size.to_be_bytes()).unwrap();
    }
}

#[async_trait]
impl FrameSender for TCPFrameSender {
    async fn send_frame(&mut self, frame_data: &mut FrameData) {
        let capture_timestamp = frame_data.get("capture_timestamp");
        let encoded_size = frame_data.get("encoded_size") as usize;
        let frame_buffer = frame_data.get_writable_buffer_ref("encoded_frame_buffer").unwrap();
        let frame_buffer = &frame_buffer[..encoded_size];

        let frame_body = FrameBody {
            capture_timestamp,
            frame_pixels: frame_buffer.to_vec(),
        };

        let binarized_obj = Bytes::from(bincode::serialize(&frame_body).unwrap());

        self.send_packet_header(binarized_obj.len());

        self.stream.write_all(&binarized_obj).unwrap();

        frame_data.set("transmitted_bytes", binarized_obj.len() as u128);
    }

    fn handle_feedback(&mut self, message: FeedbackMessage) {
        debug!("Feedback message: {:?}", message);
    }
}