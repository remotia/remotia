use std::{error::Error, fmt::Display};


#[derive(Debug)]
pub enum ClientError {
    InvalidFrameHeader,
    InvalidWholeFrameHeader,
    InvalidPacketHeader,
}

impl Error for ClientError { }

impl Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Oh no, something bad went down")
    }
}