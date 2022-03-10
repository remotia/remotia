use std::time::{Instant, Duration};

use async_trait::async_trait;
use bytes::Bytes;
use remotia::{
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
        if let Err(error) = receive_result {
            frame_data.set_drop_reason(Some(error));
            return Some(frame_data);
        }

        let (transmission_instant, binarized_obj) = receive_result.unwrap();

        let reception_delay = transmission_instant.elapsed().as_millis();
        frame_data.set("reception_delay", reception_delay);

        let serialization_result = bincode::deserialize::<SRTFrameData>(&binarized_obj);

        if let Ok(srt_frame_data) = serialization_result {
            // Convert network data to pipeline data
            srt_frame_data.merge_with_frame_data(&mut frame_data);
        } else {
            debug!("Invalid packet error: {:?}", serialization_result.unwrap_err());
            frame_data.set_drop_reason(Some(DropReason::InvalidPacket));
        }


        Some(frame_data)
    }
}
