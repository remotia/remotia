use std::{io::Write, net::TcpStream};

use super::FrameSender;

pub struct SRTFrameSender {
}

impl SRTFrameSender {
    pub fn new(
    ) -> Self {
        Self {
        }
    }

    fn send_packet_header(&mut self, frame_size: usize) {
    }
}

impl FrameSender for SRTFrameSender {
    fn send_frame(&mut self, frame_buffer: & [u8]) {
    }
}