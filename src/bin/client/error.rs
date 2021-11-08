use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Invalid whole frame header")]
    InvalidWholeFrameHeader,

    #[error("Invalid packet header")]
    InvalidPacketHeader,

    #[error("Invalid packet")]
    InvalidPacket,

    #[error("Connection error")]
    ConnectionError,
}
