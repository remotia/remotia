use std::{
    io::Write,
    net::TcpStream,
    slice::Chunks,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;

use bytes::Bytes;
use futures::{stream, SinkExt, StreamExt};

use log::{debug, info, warn};
use serde::Serialize;
use srt_tokio::{SrtSocket, SrtSocketBuilder};
use tokio::time::timeout;

use crate::{common::{feedback::FeedbackMessage, network::{FrameBody, FrameFragment, FrameHeader}}, server::error::ServerError};

use super::FrameSender;

pub struct SRTManualFragmentationFrameSender {
    socket: SrtSocket,

    timeout: Duration,
}

impl SRTManualFragmentationFrameSender {
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

    async fn send_with_timeout<T: Serialize>(&mut self, obj: T) -> Result<(), ServerError> {
        let binarized_obj = Bytes::from(bincode::serialize(&obj).unwrap());

        if let Err(_) = timeout(self.timeout, self.send_item(binarized_obj)).await {
            debug!("Timeout");
            Err(ServerError::Timeout)
        } else {
            Ok(())
        }
    }

    async fn feed_frame_header(&mut self, capture_timestamp: u128, fragments_count: usize) {
        let header = FrameHeader {
            capture_timestamp,
            fragments_count: fragments_count,
        };

        debug!("Sending frame header: {:?}", header);

        let binarized_header = bincode::serialize(&header).unwrap();

        self.socket
            .feed((Instant::now(), Bytes::from(binarized_header)))
            .await
            .unwrap();
    }

    async fn feed_frame_body(&mut self, mut chunks: Chunks<'_, u8>) {
        debug!("Sending frame body...");
        let mut fragment_idx = 1;

        while let Some(chunk) = chunks.next() {
            debug!("Sending frame fragment #{}...", fragment_idx);

            let fragment = FrameFragment {
                index: fragment_idx,
                data: chunk.to_vec(),
            };
            let binarized_fragment = bincode::serialize(&fragment).unwrap();
            self.socket
                .feed((Instant::now(), Bytes::from(binarized_fragment)))
                .await
                .unwrap();

            fragment_idx += 1;
        }

        debug!("Sent frame body");
    }

    async fn send_frame(
        &mut self,
        capture_timestamp: u128,
        frame_buffer: &[u8],
    ) -> Result<(), ServerError> {
        debug!("Sending frame...");
        let frame_data = frame_buffer.to_vec();

        let chunk_size = 256;
        let chunks = frame_data.chunks(chunk_size);

        self.feed_frame_header(capture_timestamp, chunks.len())
            .await;
        self.feed_frame_body(chunks).await;

        debug!("Flushing...");
        self.socket.flush().await.unwrap();
        debug!("Flushed");

        Ok(())
    }
}

macro_rules! phase {
    ($future: expr) => {
        if let Err(_) = $future.await {
            return 0;
        }
    };
}

#[async_trait]
impl FrameSender for SRTManualFragmentationFrameSender {
    async fn send_frame(&mut self, capture_timestamp: u128, frame_buffer: &[u8]) -> usize {
        phase!(self.send_frame(capture_timestamp, frame_buffer));
        debug!(
            "Buffer size: {}, Timestamp: {:?}",
            frame_buffer.len(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );

        frame_buffer.len()
    }

    fn handle_feedback(&mut self, message: FeedbackMessage) {
        debug!("Feedback message: {:?}", message);
    }
}
