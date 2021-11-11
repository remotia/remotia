use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Invalid whole frame header")]
    InvalidWholeFrameHeader,

    #[error("Invalid packet header")]
    InvalidPacketHeader,

    #[error("Invalid packet")]
    InvalidPacket,

    #[error("Empty frame")]
    EmptyFrame,

    #[error("Connection error")]
    ConnectionError,

    #[error("H264 Send packet error")]
    H264SendPacketError,
}
