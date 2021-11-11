use std::{cmp, net::{SocketAddr, UdpSocket}};

use super::FrameSender;

pub struct UDPFrameSender<'a> {
    socket: &'a UdpSocket,
    pixels_packet_size: usize,
    client_address: &'a SocketAddr
}

impl<'a> UDPFrameSender<'a> {
    pub fn new(
        socket: &'a UdpSocket, 
        packet_size: usize,
        client_address: &'a SocketAddr
    ) -> UDPFrameSender<'a> {
        UDPFrameSender {
            socket: socket,
            pixels_packet_size: packet_size,
            client_address: client_address
        }
    }

    

    fn send_whole_frame_header(&self) {
        println!("Sending whole frame header...");
        let frame_header = [128, 8];
        self.socket.send_to(&frame_header, self.client_address).unwrap();
        println!("Sent whole frame header.");
    }

    fn receive_whole_frame_header_receipt(&self) -> Result<(), ()> {
        println!("Waiting for whole frame header receipt...");

        let mut frame_header_receipt_buffer = [0, 8];
        let receive_result = self.socket.recv(&mut frame_header_receipt_buffer);

        if receive_result.is_err() {
            return Err(());
        }

        if frame_header_receipt_buffer[0] != 129 {
            return Err(());
        }

        println!("Received whole frame header receipt.");

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

    fn send_frame_pixels(&self, frame_buffer: &'a [u8]) {
        println!("Sending frame pixels...");

        let mut total_sent_bytes = 0;

        while total_sent_bytes < frame_buffer.len() {
            self.send_packet_header();

            let slice_end = cmp::min(total_sent_bytes+self.pixels_packet_size, frame_buffer.len());

            let packet_slice = &frame_buffer[total_sent_bytes..slice_end];

            let sent_bytes = self.socket.send_to(&packet_slice, self.client_address)
                .unwrap();

            total_sent_bytes += sent_bytes;

            // println!("Sent {}/{} bytes", total_sent_bytes, &frame_buffer.len());
        }

        println!("Sent frame pixels.");
    }
}

impl<'a> FrameSender for UDPFrameSender<'a> {
    fn send_frame(&mut self, frame_buffer: & [u8]) {
        self.send_whole_frame_header();

        if self.receive_whole_frame_header_receipt().is_err() {
            println!("Invalid whole frame header receipt, dropping frame");
            return;
        }

        self.send_frame_pixels(frame_buffer);
        self.send_end_packet_header();
    }
}