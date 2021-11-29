use std::{cmp, net::{SocketAddr, UdpSocket}};

use async_trait::async_trait;

use log::debug;

use super::FrameSender;

pub struct UDPFrameSender {
    socket: UdpSocket,
    pixels_packet_size: usize,
    client_address: SocketAddr
}

impl UDPFrameSender {
    pub fn new(
        socket: UdpSocket, 
        packet_size: usize,
        client_address: SocketAddr
    ) -> Self {
        Self {
            socket: socket,
            pixels_packet_size: packet_size,
            client_address: client_address
        }
    }

    fn send_whole_frame_header(&self) {
        debug!("Sending whole frame header...");
        let frame_header = [128, 8];
        self.socket.send_to(&frame_header, self.client_address).unwrap();
        debug!("Sent whole frame header.");
    }

    fn receive_whole_frame_header_receipt(&self) -> Result<(), ()> {
        debug!("Waiting for whole frame header receipt...");

        let mut frame_header_receipt_buffer = [0, 8];
        let receive_result = self.socket.recv(&mut frame_header_receipt_buffer);

        if receive_result.is_err() {
            return Err(());
        }

        if frame_header_receipt_buffer[0] != 129 {
            return Err(());
        }

        debug!("Received whole frame header receipt.");

        Ok(())
    }

    fn send_packet_header(&self) {
        let packet_header = [64; 8];
        self.socket.send_to(&packet_header, self.client_address)
            .unwrap();
    }

    fn send_end_packet_header(&self) {
        let packet_header = [65; 8];
        self.socket.send_to(&packet_header, self.client_address)
            .unwrap();
    }

    fn send_frame_pixels(&self, frame_buffer: &[u8]) {
        debug!("Sending frame pixels...");

        let mut total_sent_bytes = 0;

        while total_sent_bytes < frame_buffer.len() {
            self.send_packet_header();

            let slice_end = cmp::min(total_sent_bytes+self.pixels_packet_size, frame_buffer.len());

            let packet_slice = &frame_buffer[total_sent_bytes..slice_end];

            let sent_bytes = self.socket.send_to(&packet_slice, self.client_address)
                .unwrap();

            total_sent_bytes += sent_bytes;

            // debug!("Sent {}/{} bytes", total_sent_bytes, &frame_buffer.len());
        }

        debug!("Sent frame pixels.");
    }
}

#[async_trait]
impl FrameSender for UDPFrameSender {
    async fn send_frame(&mut self, frame_buffer: & [u8]) {
        self.send_whole_frame_header();

        if self.receive_whole_frame_header_receipt().is_err() {
            debug!("Invalid whole frame header receipt, dropping frame");
            return;
        }

        self.send_frame_pixels(frame_buffer);
        self.send_end_packet_header();
    }
}