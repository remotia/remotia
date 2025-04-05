use async_trait::async_trait;

use remotia_buffer_utils::BytesMut;
use remotia_core::traits::{BorrowFrameProperties, FrameProcessor};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

pub struct TcpFrameSender<K> {
    buffer_key: K,
    socket: TcpStream,
}

impl<K> TcpFrameSender<K> {
    pub fn new(buffer_key: K, socket: TcpStream) -> Self {
        Self { buffer_key, socket }
    }
}

#[async_trait]
impl<F, K> FrameProcessor<F> for TcpFrameSender<K>
where
    K: Send,
    F: BorrowFrameProperties<K, BytesMut> + Send + 'static,
{
    async fn process(&mut self, frame_data: F) -> Option<F> {
        let buffer = frame_data.get_ref(&self.buffer_key).unwrap();
        self.socket.write_all(buffer).await.unwrap();
        Some(frame_data)
    }
}
