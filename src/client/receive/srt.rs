use std::{io::Read, net::TcpStream, time::{Duration, Instant, SystemTime, UNIX_EPOCH}};

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

    timeout: Duration,
    last_receive: Instant
}

impl SRTFrameReceiver {
    pub async fn new(server_address: &str, latency: Duration, timeout: Duration) -> Self {
        info!("Connecting...");
        let socket = SrtSocketBuilder::new_connect(server_address)
            .latency(latency)
            .connect()
            .await
            .unwrap();

        info!("Connected");

        Self { socket, timeout, last_receive: Instant::now() }
    }

    async fn receive_with_timeout(&mut self) -> Result<Bytes, ClientError> {
        let receive_job = self.socket.try_next();

        match timeout(self.timeout, receive_job).await {
            Ok(packet) => {
                if let Some((_, binarized_obj)) = packet.unwrap() {
                    Ok(binarized_obj)
                } else {
                    warn!("None packet");
                    Err(ClientError::InvalidPacket)
                }
            }
            Err(_) => {
                debug!("Timeout");
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
            Ok(binarized_obj) => match bincode::deserialize::<FrameBody>(&binarized_obj) {
                Ok(body) => {
                    let frame_buffer = &mut frame_buffer[..body.frame_pixels.len()];
                    frame_buffer.copy_from_slice(&body.frame_pixels);

                    /*let frame_delay = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis()
                        - body.capture_timestamp;

                    info!("Frame delay: {}", frame_delay);*/

                    /*if frame_delay > 150 {
                        debug!("Stale frame");
                        return Err(ClientError::StaleFrame);
                    }*/

                    Ok(frame_buffer.len())
                }
                Err(err) => {
                    warn!("Corrupted body ({:?})", err);
                    Err(ClientError::InvalidPacket)
                }
            },
            Err(e) => Err(e),
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
