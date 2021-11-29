use std::{io::Write, net::TcpStream};

use async_trait::async_trait;

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
        // debug!("Sending frame header with size {}...", frame_size);
        self.stream.write_all(&frame_size.to_be_bytes()).unwrap();
    }
}

#[async_trait]
impl FrameSender for TCPFrameSender {
    async fn send_frame(&mut self, frame_buffer: & [u8]) {
        self.send_packet_header(frame_buffer.len());
        self.stream.write_all(frame_buffer).unwrap();
    }
}