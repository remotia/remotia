use std::{io::Write, net::TcpStream};

use async_trait::async_trait;
use bytes::Bytes;

use crate::common::network::FrameBody;

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
    async fn send_frame(&mut self, capture_timestamp: u128, frame_buffer: &[u8]) -> usize {
        let frame_body = FrameBody {
            capture_timestamp,
            frame_pixels: frame_buffer.to_vec(),
        };

        let binarized_obj = Bytes::from(bincode::serialize(&frame_body).unwrap());

        self.send_packet_header(binarized_obj.len());

        self.stream.write_all(&binarized_obj).unwrap();

        binarized_obj.len()
    }
}