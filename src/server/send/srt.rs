use std::{
    io::Write,
    net::TcpStream,
    time::{Duration, Instant},
};

use async_trait::async_trait;

use bytes::Bytes;
use futures::{stream, SinkExt, StreamExt};

use log::{debug, info, warn};
use serde::Serialize;
use srt_tokio::{SrtSocket, SrtSocketBuilder};
use tokio::time::timeout;

use crate::{
    common::network::{FrameBody, FrameHeader},
    server::error::ServerError,
};

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

    async fn send_with_timeout<T: Serialize>(&mut self, obj: T) -> Result<(), ServerError> {
        let binarized_obj = Bytes::from(bincode::serialize(&obj).unwrap());

        if let Err(_) = timeout(Duration::from_millis(50), self.send_item(binarized_obj)).await {
            debug!("Timeout");
            Err(ServerError::Timeout)
        } else {
            Ok(())
        }
    }

    async fn send_frame_body(&mut self, frame_buffer: &[u8]) -> Result<(), ServerError> {
        debug!("Sending frame body...");
        self.send_with_timeout(frame_buffer.to_vec()).await
    }
}

macro_rules! phase {
    ($future: expr) => {
        if let Err(_) = $future.await {
            return;
        }
    };
}

#[async_trait]
impl FrameSender for SRTFrameSender {
    async fn send_frame(&mut self, frame_buffer: &[u8]) {
        phase!(self.send_frame_body(frame_buffer));
    }
}
