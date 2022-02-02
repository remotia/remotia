use std::slice;

use log::debug;
use rsmpeg::{
    avcodec::{AVCodec, AVCodecContext, AVCodecParserContext, AVPacket},
    error::RsmpegError,
};

use cstr::cstr;

use crate::{client::error::ClientError, common::feedback::FeedbackMessage};
use async_trait::async_trait;

use super::{Decoder};

pub struct H264RGBDecoder {
    decode_context: AVCodecContext,

    parsed_offset: usize,
    parser_context: AVCodecParserContext,
}

unsafe impl Send for H264RGBDecoder { }

impl H264RGBDecoder {
    pub fn new() -> Self {
        let decoder = AVCodec::find_decoder_by_name(cstr!("h264")).unwrap();

        H264RGBDecoder {
            decode_context: {
                let mut decode_context = AVCodecContext::new(&decoder);
                decode_context.open(None).unwrap();

                decode_context
            },

            parsed_offset: 0,
            parser_context: AVCodecParserContext::find(decoder.id).unwrap(),
        }
    }
}

#[async_trait]
impl Decoder for H264RGBDecoder {
    async fn decode(
        &mut self,
        input_buffer: &[u8],
        output_buffer: &mut [u8],
    ) -> Result<usize, ClientError> {
        let mut packet = AVPacket::new();

        loop {
            let (get_packet, offset) = self
                .parser_context
                .parse_packet(
                    &mut self.decode_context,
                    &mut packet,
                    &input_buffer[self.parsed_offset..],
                )
                .unwrap();

            if get_packet {
                let result = self.decode_context.send_packet(Some(&packet));

                match result {
                    Ok(_) => (),
                    Err(e) => {
                        debug!("Error on send packet: {}", e);
                        break Err(ClientError::FFMpegSendPacketError);
                    }
                }

                loop {
                    let avframe = match self.decode_context.receive_frame() {
                        Ok(frame) => frame,
                        Err(RsmpegError::DecoderDrainError)
                        | Err(RsmpegError::DecoderFlushedError) => break,
                        Err(e) => panic!("{:?}", e),
                    };

                    let data = avframe.data;
                    let linesize = avframe.linesize;
                    let width = avframe.width as usize;
                    let height = avframe.height as usize;
                    let pixels_count = width * height;

                    let linesize_r = linesize[0] as usize;
                    let linesize_g  = linesize[1] as usize;
                    let linesize_b = linesize[2] as usize;
                    let r_data =
                        unsafe { std::slice::from_raw_parts_mut(data[0], height * linesize_r) };
                    let g_data = unsafe {
                        std::slice::from_raw_parts_mut(data[1], height * linesize_g)
                    };
                    let b_data = unsafe {
                        std::slice::from_raw_parts_mut(data[2], height * linesize_b)
                    };

                    for i in 0..pixels_count {
                        output_buffer[i * 3] = g_data[i];
                        output_buffer[i * 3 + 1] = r_data[i];
                        output_buffer[i * 3 + 2] = b_data[i];
                    }

                    
                }
            } else {
                break Ok(0);
            }

            self.parsed_offset += offset;
        }
    }

    fn handle_feedback(&mut self, message: FeedbackMessage) {
        debug!("Feedback message: {:?}", message);
    }
}
