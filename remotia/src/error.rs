use serde::{Serialize, Deserialize};
use thiserror::Error;

#[derive(Error, Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Copy)]
pub enum DropReason {
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

    #[error("Generic codec error")]
    CodecError,

    #[error("Timeout")]
    Timeout,

    #[error("NoEncodedFrames")]
    NoEncodedFrames,

    #[error("NoAvailableEncoders")]
    NoAvailableEncoders,

    #[error("No available buffers")]
    NoAvailableBuffers,
}
