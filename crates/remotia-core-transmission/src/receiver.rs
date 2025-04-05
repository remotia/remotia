use async_trait::async_trait;

use remotia_buffer_utils::BytesMut;
use remotia_core::traits::{FrameProcessor, BorrowMutFrameProperties};
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

pub struct TcpFrameReceiver<K> {
    buffer_key: K,
    socket: TcpStream,
}

impl<K> TcpFrameReceiver<K> {
    pub fn new(buffer_key: K, socket: TcpStream) -> Self {
        Self { buffer_key, socket }
    }
}

#[async_trait]
impl<F, K> FrameProcessor<F> for TcpFrameReceiver<K>
where
    K: Send,
    F: BorrowMutFrameProperties<K, BytesMut> + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        let mut buffer = frame_data.get_mut_ref(&self.buffer_key).unwrap();
        self.socket.read_exact(&mut buffer).await.unwrap();
        Some(frame_data)
    }
}
