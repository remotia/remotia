use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Serialize, Clone, PartialEq, Eq)]
pub enum ClientError {
    #[error("Invalid whole frame header")]
    InvalidWholeFrameHeader,

    #[error("Invalid packet header")]
    InvalidPacketHeader,

    #[error("Invalid packet")]
    InvalidPacket,

    #[error("Empty frame")]
    EmptyFrame,

    #[error("No frames to pull")]
    NoCompleteFrames,

    #[error("No decoded frames available")]
    NoDecodedFrames,

    #[error("Stale frame")]
    StaleFrame,

    #[error("Connection error")]
    ConnectionError,

    #[error("H264 Send packet error")]
    FFMpegSendPacketError,

    #[error("Timeout")]
    Timeout,
}
