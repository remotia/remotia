use std::net::{SocketAddr, UdpSocket};

pub struct FrameSender<'a> {
    socket: &'a UdpSocket,
    packet_size: usize,
    client_address: &'a SocketAddr
}

impl<'a> FrameSender<'a> {
    pub fn create(
        socket: &'a UdpSocket, 
        packet_size: usize,
        client_address: &'a SocketAddr
    ) -> FrameSender<'a> {
        FrameSender {
            socket: socket,
            packet_size: packet_size,
            client_address: client_address
        }
    }

    pub fn send_frame(&self, frame_buffer: &'a [u8]) {
        println!("Sending frame pixels...");
        self.send_frame_pixels(frame_buffer);
        println!("Sent frame pixels.");
    }

    fn send_frame_pixels(&self, frame_buffer: &'a [u8]) {
        let mut total_sent_bytes = 0;

        while total_sent_bytes < frame_buffer.len() {
            let packet_slice = &frame_buffer[total_sent_bytes..total_sent_bytes+self.packet_size];

            let sent_bytes = self.socket.send_to(&packet_slice, self.client_address).unwrap();

            total_sent_bytes += sent_bytes;

            println!("Sent {}/{} bytes", total_sent_bytes, &frame_buffer.len());
        }
    }
}