use std::{io::Write, net::TcpStream};

use super::FrameSender;

pub struct TCPFrameSender<'a> {
    stream: &'a mut TcpStream
}

impl<'a> TCPFrameSender<'a> {
    pub fn new(
        stream: &'a mut TcpStream
    ) -> TCPFrameSender<'a> {
        TCPFrameSender {
            stream
        }
    }

    fn send_packet_header(&mut self, frame_size: usize) {
        println!("Sending frame header with size {}...", frame_size);
        self.stream.write_all(&frame_size.to_be_bytes()).unwrap();
    }
}

impl<'a> FrameSender for TCPFrameSender<'a> {
    fn send_frame(&mut self, frame_buffer: & [u8]) {
        self.send_packet_header(frame_buffer.len());
        self.stream.write_all(frame_buffer).unwrap();
    }
}