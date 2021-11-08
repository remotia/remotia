use std::{net::UdpSocket};

pub struct FrameReceiver<'a> {
    socket: &'a UdpSocket,
}

impl<'a> FrameReceiver<'a> {
    pub fn create(socket: &'a UdpSocket) -> FrameReceiver {
        FrameReceiver {
            socket: &socket
        }
    }

    pub fn receive_frame(&self, frame_buffer: &'a mut[u8]) {
        let mut total_received_bytes = 0;

        while total_received_bytes < frame_buffer.len() {
            let packet_slice = &mut frame_buffer[total_received_bytes..];

            let received_bytes = self.socket.recv(packet_slice).unwrap();

            total_received_bytes += received_bytes;

            println!("Received {}/{} bytes", total_received_bytes, &frame_buffer.len());
        }

        println!("Received a frame (received {} bytes)", total_received_bytes);
    }
}