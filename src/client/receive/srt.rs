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

use crate::{
    client::error::ClientError,
    common::{
        feedback::FeedbackMessage,
        network::{FrameBody, FrameHeader},
    },
};

use super::{FrameReceiver, ReceivedFrame};

pub struct SRTFrameReceiver {
    socket: SrtSocket
}

impl SRTFrameReceiver {
    pub async fn new(server_address: &str) -> Self {
        info!("Connecting...");
        let socket = SrtSocket::builder()
            .call(server_address, None)
            .await
            .unwrap();

        info!("Connected");

        Self {
            socket
        }
    }

    async fn receive_frame_pixels(
        &mut self,
        frame_buffer: &mut [u8],
    ) -> Result<ReceivedFrame, ClientError> {
        debug!("Receiving encoded frame bytes...");

        let receive_result = self.socket.try_next().await;

        if let Err(error) = receive_result {
            debug!("Connection error: {:?}", error);
            return Err(ClientError::ConnectionError);
        }

        let receive_result = receive_result.unwrap();

        if receive_result.is_none() {
            debug!("None receive result");
            return Err(ClientError::EmptyFrame);
        }

        let (instant, binarized_obj) = receive_result.unwrap();

        match bincode::deserialize::<FrameBody>(&binarized_obj) {
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
                    reception_delay,
                })
            }
            Err(error) => {
                debug!("Invalid packet error: {:?}", error);
                return Err(ClientError::InvalidPacket);
            },
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
