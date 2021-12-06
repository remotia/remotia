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
    common::network::{FrameBody, FrameFragment, FrameHeader},
};

use super::{FrameReceiver, ReceivedFrame};

pub struct SRTManualFragmentationFrameReceiver {
    socket: SrtSocket,

    timeout: Duration,
    last_receive: Instant,
}

impl SRTManualFragmentationFrameReceiver {
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

    async fn receive_frame_header(&mut self) -> (Instant, FrameHeader) {
        debug!("Receiving frame header...");

        let (header_reception_instant, binarized_header) =
            self.socket.try_next().await.unwrap().unwrap();

        let frame_header = bincode::deserialize::<FrameHeader>(&binarized_header).unwrap();

        debug!("Done: {:?}", frame_header);

        (header_reception_instant, frame_header)
    }

    async fn receive_frame_body(
        &mut self,
        frame_buffer: &mut [u8],
        fragments_count: usize,
    ) -> usize {
        debug!("Receiving frame body...");

        let mut received_fragments = 0;
        let mut chunk_start_idx = 0;

        while received_fragments < fragments_count {
            debug!("Receiving frame fragment {}/{}...", received_fragments, fragments_count);

            let (_, binarized_fragment) = self.socket.try_next().await.unwrap().unwrap();
            let fragment = bincode::deserialize::<FrameFragment>(&binarized_fragment).unwrap();

            debug!("Fragment index: {}", fragment.index);

            let chunk_end_idx = chunk_start_idx + fragment.data.len();
            frame_buffer[chunk_start_idx..chunk_end_idx].copy_from_slice(&fragment.data);

            chunk_start_idx = chunk_end_idx;

            received_fragments += 1;
        }

        chunk_start_idx 
    }

    async fn receive_frame(
        &mut self,
        frame_buffer: &mut [u8],
    ) -> Result<ReceivedFrame, ClientError> {
        let (header_reception_instant, frame_header) = self.receive_frame_header().await;
        let reception_delay = header_reception_instant.elapsed().as_millis();
        let capture_timestamp = frame_header.capture_timestamp;

        let buffer_size = self
            .receive_frame_body(frame_buffer, frame_header.fragments_count)
            .await;

        Ok(ReceivedFrame {
            buffer_size,
            capture_timestamp,
            reception_delay,
        })
    }
}

#[async_trait]
impl FrameReceiver for SRTManualFragmentationFrameReceiver {
    async fn receive_encoded_frame(
        &mut self,
        frame_buffer: &mut [u8],
    ) -> Result<ReceivedFrame, ClientError> {
        self.receive_frame(frame_buffer).await
    }
}
