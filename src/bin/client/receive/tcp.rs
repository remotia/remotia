use std::{io::Read, net::{TcpStream}, ptr::read};

use crate::error::ClientError;

use super::FrameReceiver;

pub struct TCPFrameReceiver<'a> {
    stream: &'a mut TcpStream,
}

impl<'a> TCPFrameReceiver<'a> {
    pub fn create(
        stream: &'a mut TcpStream,
    ) -> TCPFrameReceiver<'a> {
        TCPFrameReceiver {
            stream
        }
    }
}

impl<'a> FrameReceiver for TCPFrameReceiver<'a> {
    fn receive_frame(&mut self, frame_buffer: &mut[u8]) -> Result<(), ClientError> {
        let read_bytes = self.stream.read(frame_buffer).unwrap();

        if read_bytes == 0 {
            return Err(ClientError::InvalidPacket);
        }

        Ok(())
    }
}