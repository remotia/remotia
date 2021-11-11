use std::{net::{SocketAddr, UdpSocket}};

use crate::error::ClientError;

use super::FrameReceiver;

pub struct UDPFrameReceiver<'a> {
    socket: &'a UdpSocket,
    server_address: &'a SocketAddr
}

impl<'a> UDPFrameReceiver<'a> {
    pub fn create(
        socket: &'a UdpSocket,
        server_address: &'a SocketAddr
    ) -> UDPFrameReceiver<'a> {
        UDPFrameReceiver {
            socket: socket,
            server_address: server_address
        }
    }

    fn receive_whole_frame_header(&self) -> Result<(), ClientError> {
        println!("Receiving whole frame header...");
        let mut frame_header_buffer = [0, 8];
        let receive_result = self.socket.recv(&mut frame_header_buffer);

        if receive_result.is_err() {
            println!("Couldn't receive whole frame header, connection error.");
            return Err(ClientError::ConnectionError);
        }

        println!("Received whole frame header.");

        if frame_header_buffer[0] != 128 {
            println!("Invalid whole frame header, dropping frame.");
            return Err(ClientError::InvalidWholeFrameHeader);
        }

        Ok(())
    }

    fn send_whole_frame_header_receipt(&self) {
        println!("Sending whole frame header receipt...");
        let frame_header_receipt = [129, 8];
        self.socket.send_to(&frame_header_receipt, self.server_address).unwrap();
        println!("Sent whole frame header receipt.");
    }

    fn receive_packet_header(&self) -> Result<bool, ClientError> {
        let mut packet_header_buffer = [0; 8];
        let receive_result = self.socket.recv(&mut packet_header_buffer);

        if receive_result.is_err() {
            println!("Couldn't receive packet header, connection error.");
            return Err(ClientError::ConnectionError);
        }

        if packet_header_buffer[0] != 64 {
            if packet_header_buffer[0] == 65 {
                println!("Received end packet header");
                return Ok(true);
            }
            println!("Invalid packet header");
            return Err(ClientError::InvalidPacketHeader);
        }

        Ok(false)
    }

    fn receive_frame_pixels(&self, frame_buffer: &'a mut[u8]) -> Result<(), ClientError> {
        println!("Receiving frame pixels...");

        let mut total_received_bytes = 0;

        // while total_received_bytes < frame_buffer.len() {
        loop {
            let end = self.receive_packet_header()?;

            if end {
                break;
            }

            let packet_slice = &mut frame_buffer[total_received_bytes..];

            let received_bytes = match self.socket.recv(packet_slice) {
                Ok(value) => value,
                Err(e) => {
                    println!("Receive error: {}", e);
                    return Err(ClientError::InvalidPacket);
                }
            };

            if received_bytes == 0 {
                break;
            }

            total_received_bytes += received_bytes;

            // println!("Received {}/{} bytes", total_received_bytes, &frame_buffer.len());
            // println!("Received {} bytes", total_received_bytes);
        }

        println!("Received frame pixels (received {} bytes)", total_received_bytes);

        Ok(())
    }

}

impl<'a> FrameReceiver for UDPFrameReceiver<'a> {
    fn receive_frame(&mut self, frame_buffer: &mut[u8]) -> Result<(), ClientError> {
        self.receive_whole_frame_header()?;
        self.send_whole_frame_header_receipt();
        self.receive_frame_pixels(frame_buffer)?;
        Ok(())
    }
}