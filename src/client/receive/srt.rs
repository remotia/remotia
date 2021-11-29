use std::{
    io::Read,
    net::TcpStream,
    time::{Duration, Instant},
};

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

        match timeout(Duration::from_millis(50), receive_job).await {
            Ok(packet) => {
                if let Some((_, binarized_obj)) = packet.unwrap() {
                    Ok(binarized_obj)
                } else {
                    warn!("None packet");
                    Err(ClientError::InvalidPacket)
                }
            }
            Err(_) => {
                warn!("Timeout");
                return Err(ClientError::Timeout);
            }
        }
    }

    async fn receive_frame_pixels(
        &mut self,
        frame_buffer: &mut [u8],
    ) -> Result<usize, ClientError> {
        debug!("Receiving encoded frame bytes...");

        match self.receive_with_timeout().await {
            Ok(binarized_obj) => {
                match bincode::deserialize::<Vec<u8>>(&binarized_obj) {
                    Ok(body) => {
                        let frame_buffer = &mut frame_buffer[..body.len()];
                        frame_buffer.copy_from_slice(&body);
                        Ok(frame_buffer.len())
                    }
                    Err(err) => {
                        warn!("Corrupted body ({:?})", err);
                        Err(ClientError::InvalidPacket)
                    }
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
        self.receive_frame_pixels(frame_buffer).await
    }
}
