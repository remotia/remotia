use std::{io::Read, net::{TcpStream}};

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

    fn receive_frame_header(&mut self) -> usize {
        println!("Receiving frame header...");

        let mut frame_size_vec = [0 as u8; 8];

        self.stream.read(&mut frame_size_vec).unwrap();

        println!("Frame size: {:?}", frame_size_vec);

        usize::from_be_bytes(frame_size_vec)
    }

    fn receive_frame_pixels(&mut self, frame_buffer: &mut[u8])  -> Result<usize, ClientError> {
        println!("Receiving {} encoded frame bytes...", frame_buffer.len());

        let mut total_read_bytes = 0;

        while total_read_bytes < frame_buffer.len() {
            let read_bytes = self.stream.read(&mut frame_buffer[total_read_bytes..]).unwrap();
            println!("Received {} bytes", read_bytes); 

            if read_bytes == 0 {
                return Err(ClientError::InvalidPacket);
            }

            total_read_bytes += read_bytes;
        }

        Ok(total_read_bytes)
    }
}

impl<'a> FrameReceiver for TCPFrameReceiver<'a> {
    fn receive_encoded_frame(&mut self, frame_buffer: &mut[u8]) -> Result<usize, ClientError> {
        let frame_size = self.receive_frame_header();

        self.receive_frame_pixels(&mut frame_buffer[..frame_size])
    }
}