use std::{io::Read, net::TcpStream};

use async_trait::async_trait;

use futures::TryStreamExt;
use log::{debug, info, warn};
use srt_tokio::{SrtSocket, SrtSocketBuilder};

use crate::{
    client::error::ClientError,
    common::network::{FrameBody, FrameHeader},
};

use super::FrameReceiver;

pub struct SRTFrameReceiver {
    socket: SrtSocket,
}

impl SRTFrameReceiver {
    pub async fn new(server_address: &str) -> Self {
        info!("Connecting...");
        let srt_socket = SrtSocketBuilder::new_connect(server_address)
            .connect()
            .await
            .unwrap();

        info!("Connected");

        Self { socket: srt_socket }
    }

    async fn receive_frame_header(&mut self) -> Result<usize, ClientError> {
        debug!("Receiving frame header...");

        match self.socket.try_next().await {
            Ok(received_packet) => {
                if let Some((_, binarized_header)) = received_packet {
                    let header: FrameHeader = bincode::deserialize(&binarized_header).unwrap();
                    Ok(header.frame_size)
                } else {
                    warn!("None packet");
                    Err(ClientError::InvalidPacketHeader)
                }
            }
            Err(_e) => Err(ClientError::InvalidPacketHeader),
        }
    }

    async fn receive_frame_pixels(
        &mut self,
        frame_buffer: &mut [u8],
    ) -> Result<usize, ClientError> {
        debug!("Receiving {} encoded frame bytes...", frame_buffer.len());

        match self.socket.try_next().await {
            Ok(received_packet) => {
                let read_bytes = if let Some((_, binarized_body)) = received_packet {
                    let body: FrameBody = bincode::deserialize(&binarized_body).unwrap();
                    frame_buffer.copy_from_slice(body.frame_pixels);
                    frame_buffer.len()
                } else {
                    warn!("None packet");
                    0
                };

                Ok(read_bytes)
            }
            Err(_e) => Err(ClientError::InvalidPacket),
        }
    }
}

#[async_trait]
impl FrameReceiver for SRTFrameReceiver {
    async fn receive_encoded_frame(
        &mut self,
        frame_buffer: &mut [u8],
    ) -> Result<usize, ClientError> {
        let frame_size = self.receive_frame_header().await?;
        self.receive_frame_pixels(&mut frame_buffer[..frame_size])
            .await
    }
}
