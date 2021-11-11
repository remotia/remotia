use std::slice;

use rsmpeg::{
    avcodec::{AVCodec, AVCodecContext, AVCodecParserContext, AVPacket},
    error::RsmpegError,
};

use cstr::cstr;

use crate::error::ClientError;

use super::Decoder;

pub struct H264Decoder {
    decoded_frame_buffer: Vec<u8>,
    decode_context: AVCodecContext,

    parsed_offset: usize,
    parser_context: AVCodecParserContext,
}

impl H264Decoder {
    pub fn new(width: usize, height: usize) -> Self {
        let frame_buffer_size = width * height * 3;

        let decoder = AVCodec::find_decoder_by_name(cstr!("h264")).unwrap();

        H264Decoder {
            decoded_frame_buffer: vec![0 as u8; frame_buffer_size],
            decode_context: AVCodecContext::new(&decoder),

            parsed_offset: 0,
            parser_context: AVCodecParserContext::find(decoder.id).unwrap(),
        }
    }
}

impl Decoder for H264Decoder {
    fn decode(&mut self, encoded_frame_buffer: &[u8]) -> Result<usize, ClientError> {
        let mut packet = AVPacket::new();

        loop {
            let (get_packet, offset) = self
                .parser_context
                .parse_packet(
                    &mut self.decode_context,
                    &mut packet,
                    &encoded_frame_buffer[self.parsed_offset..],
                )
                .unwrap();

            if get_packet {
                let result = self.decode_context.send_packet(Some(&packet));

                match result {
                    Ok(_) => (),
                    Err(e) => {
                        break Err(ClientError::H264SendPacketError);
                    }
                }

                loop {
                    let avframe = match self.decode_context.receive_frame() {
                        Ok(frame) => frame,
                        Err(RsmpegError::DecoderDrainError)
                        | Err(RsmpegError::DecoderFlushedError) => break,
                        Err(e) => panic!("{:?}", e),
                    };

                    let data = avframe.data[0];
                    let linesize = avframe.linesize[0] as usize;

                    // let width = avframe.width as usize;
                    let height = avframe.height as usize;

                    let frame_slice =
                        unsafe { slice::from_raw_parts(data, linesize * height) };

                    self.decoded_frame_buffer.copy_from_slice(frame_slice);
                }
            } else {
                break Ok(0);
            }

            self.parsed_offset += offset;
        }
    }

    fn get_decoded_frame(&self) -> &[u8] {
        self.decoded_frame_buffer.as_slice()
    }
}
