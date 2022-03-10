use std::time::{Instant, Duration};

use async_trait::async_trait;

use bytes::{Bytes};
use futures::SinkExt;

use log::{debug, info};
use remotia::{
    traits::FrameProcessor,
    types::FrameData,
};
use srt_tokio::{
    options::{ByteCount, PacketSize},
    SrtSocket,
};

use crate::SRTFrameData;

pub struct SRTFrameSender {
    socket: SrtSocket,
}

impl SRTFrameSender {
    pub async fn new(port: u16, latency: Duration) -> Self {
        info!("Listening...");
        let socket = SrtSocket::builder()
            .set(|options| {
                options.sender.buffer_size = ByteCount(1024 * 1024 * 32); // 32 MB for internal buffering
                options.sender.max_payload_size = PacketSize(1024 * 1024 * 32);
            })
            .latency(latency)
            .listen_on(port)
            .await
            .unwrap();

        info!("Connected");

        Self { socket }
    }

    async fn send_frame_data(&mut self, frame_data: &mut FrameData) {
        // Create the network DTO
        let srt_frame_data = SRTFrameData::from_frame_data(frame_data);

        debug!("Sending frame body...");
        let binarized_obj = Bytes::from(bincode::serialize(&srt_frame_data).unwrap());

        self.socket
            .send((Instant::now(), binarized_obj))
            .await
            .unwrap();
    }
}

#[async_trait]
impl FrameProcessor for SRTFrameSender {
    async fn process(&mut self, mut frame_data: FrameData) -> Option<FrameData> {
        self.send_frame_data(&mut frame_data).await;
        Some(frame_data)
    }
}
