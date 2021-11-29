use std::{io::Read, net::TcpStream, time::{Duration, Instant}};

use async_trait::async_trait;
use bytes::Bytes;
use tokio::time::timeout;

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
            .latency(Duration::from_millis(10))
            .connect()
            .await
            .unwrap();

        info!("Connected");

        Self { socket: srt_socket }
    }

    async fn receive_with_timeout(&mut self) -> Result<Bytes, ClientError> {
        let receive_job = self.socket.try_next();

        match timeout(Duration::from_secs(1), receive_job).await {
            Ok(packet) => {
                if let Some((_, binarized_obj)) = packet.unwrap() {
                    Ok(binarized_obj)
                } else {
                    warn!("None packet");
                    Err(ClientError::InvalidPacket)
                }
            },
            Err(_) => {
                warn!("Timeout");
                return Err(ClientError::Timeout);
            }
        }
    }

    async fn receive_frame_header(&mut self) -> Result<usize, ClientError> {
        debug!("Receiving frame header...");

        match self.receive_with_timeout().await {
            Ok(binarized_obj) => {
                if let Ok(header) = bincode::deserialize::<FrameHeader>(&binarized_obj) {
                    Ok(header.frame_size)
                } else {
                    warn!("Corrupted header");
                    Err(ClientError::InvalidPacketHeader)
                }
            }
            Err(_) => Err(ClientError::InvalidPacketHeader),
        }
    }

    async fn receive_frame_pixels(
        &mut self,
        frame_buffer: &mut [u8],
    ) -> Result<usize, ClientError> {
        debug!("Receiving {} encoded frame bytes...", frame_buffer.len());

        match self.receive_with_timeout().await {
            Ok(binarized_obj) => {
                if let Ok(body) = bincode::deserialize::<FrameBody>(&binarized_obj) {
                    frame_buffer.copy_from_slice(body.frame_pixels);
                    Ok(frame_buffer.len())
                } else {
                    warn!("Corrupted body");
                    Err(ClientError::InvalidPacket)
                }
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
