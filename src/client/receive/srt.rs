use std::{io::Read, net::TcpStream};

use async_trait::async_trait;

use futures::TryStreamExt;
use log::{debug, warn};
use srt_tokio::{SrtSocket, SrtSocketBuilder};

use crate::{client::error::ClientError, common::network::FrameHeader};

use super::FrameReceiver;

pub struct SRTFrameReceiver {
    socket: SrtSocket,
}

impl SRTFrameReceiver {
    pub async fn new(server_address: &str) -> Self {
        let srt_socket = SrtSocketBuilder::new_connect(server_address)
            .connect()
            .await
            .unwrap();

        Self { socket: srt_socket }
    }

    async fn receive_frame_header(&mut self) -> Result<usize, ClientError> {
        debug!("Receiving frame header...");

        match self.socket.try_next().await {
            Ok(received_packet) => {
                let frame_size = if let Some((_, binarized_header)) = received_packet {
                    let header: FrameHeader = bincode::deserialize(&binarized_header).unwrap();
                    header.frame_size
                } else {
                    warn!("None packet");
                    0
                };

                Ok(frame_size)
            }
            Err(e) => Err(ClientError::InvalidPacketHeader),
        }
    }

    fn receive_frame_pixels(&mut self, frame_buffer: &mut [u8]) -> Result<usize, ClientError> {
        debug!("Receiving {} encoded frame bytes...", frame_buffer.len());

        let mut total_read_bytes = 0;

        Ok(total_read_bytes)
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
    }
}
