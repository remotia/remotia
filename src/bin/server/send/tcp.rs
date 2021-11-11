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
}

impl<'a> FrameSender for TCPFrameSender<'a> {
    fn send_frame(&mut self, frame_buffer: & [u8]) {
        self.stream.write_all(frame_buffer).unwrap();
    }
}