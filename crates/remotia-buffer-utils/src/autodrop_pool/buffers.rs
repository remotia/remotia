use bytes::BytesMut;

use tokio::sync::mpsc::Sender;

pub struct AutodroppingBuffer {
    pub(super) data: BytesMut,
    pub(super) sender: Sender<Self>,
}
