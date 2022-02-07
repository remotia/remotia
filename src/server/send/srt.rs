use std::{
    io::Write,
    net::TcpStream,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;

use bytes::{Bytes, BytesMut};
use futures::{stream, SinkExt, StreamExt};

use log::{debug, info, warn};
use serde::Serialize;
use srt_tokio::{
    options::{ByteCount, PacketSize},
    SrtSocket, SrtSocketBuilder,
};
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
}

impl SRTFrameSender {
    pub async fn new(port: u16) -> Self {
        info!("Listening...");
        let socket = SrtSocket::builder()
            .set(|options| {
                options.sender.buffer_size = ByteCount(1024 * 1024 * 32); // 32 MB for internal buffering
                options.sender.max_payload_size = PacketSize(1024 * 1024 * 32);
            })
            .listen_on(port)
            .await
            .unwrap();

        info!("Connected");

        Self { socket }
    }
}

#[async_trait]
impl FrameSender for SRTFrameSender {
    async fn send_frame(&mut self, frame_data: &mut ServerFrameData) {
        let capture_timestamp = frame_data.get("capture_timestamp");

        // Extract the slice of the encoded buffer which contains data to be transmitted
        let encoded_size = frame_data.get("encoded_size") as usize;
        let full_frame_buffer = frame_data
            .get_writable_buffer_ref("encoded_frame_buffer")
            .unwrap();

        // Copy frame data to a local frame buffer
        let mut frame_buffer = BytesMut::new(); // full_frame_buffer.split_to(encoded_size);
        frame_buffer.resize(encoded_size, 0);
        frame_buffer.copy_from_slice(&full_frame_buffer[..encoded_size]);

        debug!("Sending frame body...");
        let obj = FrameBody {
            capture_timestamp,
            frame_pixels: frame_buffer.to_vec(),
        };
        let binarized_obj = Bytes::from(bincode::serialize(&obj).unwrap());

        self.socket
            .send((Instant::now(), binarized_obj))
            .await
            .unwrap();
    }

    fn handle_feedback(&mut self, message: FeedbackMessage) {
        debug!("Feedback message: {:?}", message);
    }
}
