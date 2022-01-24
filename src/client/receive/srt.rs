use std::{
    io::Read,
    net::TcpStream,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use bytes::Bytes;
use tokio::time::timeout;

use futures::TryStreamExt;
use log::{debug, info, warn};
use srt_tokio::{SrtSocket, SrtSocketBuilder};

use crate::{client::error::ClientError, common::{feedback::FeedbackMessage, network::{FrameBody, FrameHeader}}};

use super::{FrameReceiver, ReceivedFrame};

pub struct SRTFrameReceiver {
    socket: SrtSocket,

    timeout: Duration,
    last_receive: Instant,
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

        Self {
            socket,
            timeout,
            last_receive: Instant::now(),
        }
    }

    async fn receive_with_timeout(&mut self) -> Result<(Instant, Bytes), ClientError> {
        let receive_job = self.socket.try_next();

        match timeout(self.timeout, receive_job).await {
            Ok(packet) => {
                if let Some((instant, binarized_obj)) = packet.unwrap() {
                    Ok((instant, binarized_obj))
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
    ) -> Result<ReceivedFrame, ClientError> {
        debug!("Receiving encoded frame bytes...");

        match self.receive_with_timeout().await {
            Ok((instant, binarized_obj)) => match bincode::deserialize::<FrameBody>(&binarized_obj) {
                Ok(body) => {
                    let frame_buffer = &mut frame_buffer[..body.frame_pixels.len()];
                    frame_buffer.copy_from_slice(&body.frame_pixels);

                    let reception_delay = instant.elapsed().as_millis();

                    debug!(
                        "Received buffer size: {}, Timestamp: {:?}, Reception delay: {}",
                        frame_buffer.len(),
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_millis(),
                        reception_delay
                    );

                    Ok(ReceivedFrame {
                        buffer_size: frame_buffer.len(),
                        capture_timestamp: body.capture_timestamp,
                        reception_delay
                    })
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
    ) -> Result<ReceivedFrame, ClientError> {
        self.receive_frame_pixels(frame_buffer).await
    }

    fn handle_feedback(&mut self, message: FeedbackMessage) {
        debug!("Feedback message: {:?}", message);
    }
}
