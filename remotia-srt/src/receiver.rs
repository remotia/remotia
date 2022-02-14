use std::time::{SystemTime, UNIX_EPOCH, Instant, Duration};

use async_trait::async_trait;
use bytes::Bytes;
use remotia::{
    client::receive::{FrameReceiver, ReceivedFrame},
    common::{feedback::FeedbackMessage, network::FrameBody},
    traits::FrameProcessor,
    types::FrameData,
};

use futures::TryStreamExt;
use log::{debug, info};
use srt_tokio::SrtSocket;

use remotia::error::DropReason;

use crate::SRTFrameData;

pub struct SRTFrameReceiver {
    socket: SrtSocket,
}

impl SRTFrameReceiver {
    pub async fn new(server_address: &str, latency: Duration) -> Self {
        info!("Connecting...");
        let socket = SrtSocket::builder()
            .latency(latency)
            .call(server_address, None)
            .await
            .unwrap();

        info!("Connected");

        Self { socket }
    }

    async fn receive_frame_pixels(
        &mut self,
        frame_buffer: &mut [u8],
    ) -> Result<ReceivedFrame, DropReason> {
        debug!("Receiving encoded frame bytes...");

        let receive_result = self.receive_binarized().await;
        if receive_result.is_err() {
            return Err(receive_result.unwrap_err());
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
                return Err(DropReason::InvalidPacket);
            }
        }
    }

    async fn receive_binarized(&mut self) -> Result<(Instant, Bytes), DropReason> {
        let receive_result = self.socket.try_next().await;

        if let Err(error) = receive_result {
            debug!("Connection error: {:?}", error);
            return Err(DropReason::ConnectionError);
        }

        let receive_result = receive_result.unwrap();

        if receive_result.is_none() {
            debug!("None receive result");
            return Err(DropReason::EmptyFrame);
        }

        Ok(receive_result.unwrap())
    }
}

#[async_trait]
impl FrameProcessor for SRTFrameReceiver {
    async fn process(&mut self, mut frame_data: FrameData) -> Option<FrameData> {
        debug!("Receiving binarized frame DTO...");

        let receive_result = self.receive_binarized().await;
        if receive_result.is_err() {
            frame_data.set_drop_reason(Some(receive_result.unwrap_err()));
            return Some(frame_data);
        }

        let (transmission_instant, binarized_obj) = receive_result.unwrap();

        let reception_delay = transmission_instant.elapsed().as_millis();
        frame_data.set("reception_delay", reception_delay);

        let serialization_result = bincode::deserialize::<SRTFrameData>(&binarized_obj);

        if serialization_result.is_ok() {
            let srt_frame_data = serialization_result.unwrap();

            // Convert network data to pipeline data
            srt_frame_data.merge_with_frame_data(&mut frame_data);
        } else {
            debug!("Invalid packet error: {:?}", serialization_result.unwrap_err());
            frame_data.set_drop_reason(Some(DropReason::InvalidPacket));
        }


        Some(frame_data)
    }
}

// retro-compatibility with silo pipeline
#[async_trait]
impl FrameReceiver for SRTFrameReceiver {
    async fn receive_encoded_frame(
        &mut self,
        frame_buffer: &mut [u8],
    ) -> Result<ReceivedFrame, DropReason> {
        self.receive_frame_pixels(frame_buffer).await
    }

    fn handle_feedback(&mut self, message: FeedbackMessage) {
        debug!("Feedback message: {:?}", message);
    }
}
