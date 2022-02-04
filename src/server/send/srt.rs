use std::{
    io::Write,
    net::TcpStream,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;

use bytes::Bytes;
use futures::{stream, SinkExt, StreamExt};

use log::{debug, info, warn};
use serde::Serialize;
use srt_tokio::{SrtSocket, SrtSocketBuilder};
use tokio::time::timeout;

use crate::{
    common::{
        feedback::FeedbackMessage,
        network::{FrameBody, FrameHeader},
    },
    server::{error::ServerError, types::ServerFrameData},
};

use super::FrameSender;

pub struct SRTFrameSender {
    socket: SrtSocket,

    timeout: Duration,
}

impl SRTFrameSender {
    pub async fn new(port: u16, latency: Duration, timeout: Duration) -> Self {
        info!("Listening...");
        let socket = SrtSocketBuilder::new_listen()
            .latency(latency)
            .local_port(port)
            .connect()
            .await
            .unwrap();

        info!("Connected");

        Self { socket, timeout }
    }

    async fn send_item(&mut self, binarized_item: Bytes) {
        self.socket
            .send((Instant::now(), binarized_item))
            .await
            .unwrap();
    }

    async fn send_frame_body(
        &mut self,
        capture_timestamp: u128,
        frame_buffer: &[u8],
    ) -> Result<(), ServerError> {
        debug!("Sending frame body...");
        let obj = FrameBody {
            capture_timestamp,
            frame_pixels: frame_buffer.to_vec(),
        };
        let binarized_obj = Bytes::from(bincode::serialize(&obj).unwrap());
        self.send_item(binarized_obj).await;

        Ok(())
    }
}

#[async_trait]
impl FrameSender for SRTFrameSender {
    async fn send_frame(&mut self, frame_data: &mut ServerFrameData) {
        let capture_timestamp = frame_data.get("capture_timestamp");

        // Extract the slice of the encoded buffer which contains data to be transmitted
        let encoded_size = frame_data.get("encoded_size") as usize;
        let mut full_frame_buffer = frame_data
            .extract_writable_buffer("encoded_frame_buffer")
            .unwrap();
        let mut frame_buffer = full_frame_buffer.split_to(encoded_size);

        self.send_frame_body(capture_timestamp, &frame_buffer).await.unwrap();
        debug!(
            "Buffer size: {}, Timestamp: {:?}",
            frame_buffer.len(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );

        frame_data.set_local("transmitted_bytes", frame_buffer.len() as u128);

        // Put the whole buffer back into the DTO such that the pipeline may return the buffer
        frame_buffer.unsplit(full_frame_buffer);
        frame_data.insert_writable_buffer("encoded_frame_buffer", frame_buffer);
    }

    fn handle_feedback(&mut self, message: FeedbackMessage) {
        debug!("Feedback message: {:?}", message);
    }
}
