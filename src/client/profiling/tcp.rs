use std::io::Write;

use tokio::{io::AsyncWriteExt, net::TcpStream};

use crate::common::feedback::FeedbackMessage;

use super::{ClientProfiler, ReceivedFrameStats};

use async_trait::async_trait;

pub struct TCPClientProfiler {
    socket: TcpStream,
}

impl TCPClientProfiler {
    pub async fn connect() -> Self {
        let socket = TcpStream::connect("127.0.0.1:5011").await.unwrap();

        Self { socket }
    }
}

#[async_trait]
impl ClientProfiler for TCPClientProfiler {
    async fn profile_frame(&mut self, frame_stats: ReceivedFrameStats) -> Option<FeedbackMessage> {
        let mut message: Option<FeedbackMessage> = None;

        if frame_stats.frame_delay > 200 {
            message = Some(FeedbackMessage::HighFrameDelay(
                frame_stats.frame_delay,
            ));
        }

        if let Some(message) = message {
            let binarized_message = bincode::serialize(&message).unwrap();

            self.socket.write(&binarized_message).await.unwrap();
        }

        message
    }
}
