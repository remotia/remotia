use log::debug;
use rsmpeg::{avcodec::AVCodecContext, avutil::AVFrame, error::RsmpegError};

use super::{yuv420p::YUV420PEncoder, Encoder};

pub mod frame_builders;

pub mod h264;
pub mod h264rgb;
pub mod h265;

pub struct FFMpegEncodingBridge {
    encoded_frame_buffer: Vec<u8>,
    encoded_frame_length: usize,
}

impl FFMpegEncodingBridge {
    pub fn new(frame_buffer_size: usize) -> Self {
        FFMpegEncodingBridge {
            encoded_frame_buffer: vec![0 as u8; frame_buffer_size],
            encoded_frame_length: 0,
        }
    }

    pub fn encode_avframe(
        &mut self,
        encode_context: &mut AVCodecContext,
        avframe: AVFrame
    ) -> usize {
        self.encoded_frame_length = 0;

        encode_context.send_frame(Some(&avframe)).unwrap();

        loop {
            let packet = match encode_context.receive_packet() {
                Ok(packet) => {
                    // debug!("Received packet of size {}", packet.size);
                    packet
                }
                Err(RsmpegError::EncoderDrainError) => {
                    debug!("Drain error, breaking the loop");
                    break;
                }
                Err(RsmpegError::EncoderFlushedError) => {
                    debug!("Flushed error, breaking the loop");
                    break;
                }
                Err(e) => panic!("{:?}", e),
            };

            let data = unsafe { std::slice::from_raw_parts(packet.data, packet.size as usize) };

            let start_index = self.encoded_frame_length;
            let end_index = self.encoded_frame_length + data.len();

            self.encoded_frame_buffer[start_index..end_index].copy_from_slice(data);

            self.encoded_frame_length = end_index;
        }

        self.encoded_frame_length as usize
    }

    fn get_encoded_frame(&self) -> &[u8] {
        &self.encoded_frame_buffer.as_slice()[..self.encoded_frame_length]
    }
}
