use std::{io::Write, net::TcpStream, time::{Duration, Instant}};

use async_trait::async_trait;

use bytes::Bytes;
use futures::{stream, SinkExt, StreamExt};

use log::{debug, info};
use srt_tokio::{SrtSocket, SrtSocketBuilder};

use crate::common::network::{FrameBody, FrameHeader};

use super::FrameSender;

pub struct SRTFrameSender {
    socket: SrtSocket,
}

impl SRTFrameSender {
    pub async fn new(port: u16) -> Self {
        info!("Listening...");
        let srt_socket = SrtSocketBuilder::new_listen()
            .latency(Duration::from_millis(10))
            .local_port(port)
            .connect()
            .await
            .unwrap();

        info!("Connected");

        Self { socket: srt_socket }
    }

    async fn send_item(&mut self, binarized_item: Bytes) {
        self.socket
            .send((Instant::now(), binarized_item))
            .await
            .unwrap();
    }

    async fn send_frame_header(&mut self, frame_size: usize) {
        let header = FrameHeader { frame_size };
        debug!("Sending frame header: {:?}", header);
        let binarized_header = Bytes::from(bincode::serialize(&header).unwrap());
        self.send_item(binarized_header).await;
    }

    async fn send_frame_body(&mut self, frame_buffer: &[u8]) {
        debug!("Sending frame body...");
        let body = FrameBody {
            frame_pixels: frame_buffer,
        };
        let binarized_body = Bytes::from(bincode::serialize(&body).unwrap());
        self.send_item(binarized_body).await;
    }
}

#[async_trait]
impl FrameSender for SRTFrameSender {
    async fn send_frame(&mut self, frame_buffer: &[u8]) {
        self.send_frame_header(frame_buffer.len()).await;
        self.send_frame_body(frame_buffer).await;
    }
}
