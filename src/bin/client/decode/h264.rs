use std::slice;

use log::debug;
use rsmpeg::{
    avcodec::{AVCodec, AVCodecContext, AVCodecParserContext, AVPacket},
    error::RsmpegError,
};

use cstr::cstr;
use yuv::{
    color::{MatrixCoefficients, Range},
    convert::RGBConvert,
    YUV,
};

use crate::error::ClientError;

use super::{Decoder, yuv420p::YUV420PDecoder};

pub struct H264Decoder {
    decoded_frame_buffer: Vec<u8>,
    decode_context: AVCodecContext,

    parsed_offset: usize,
    parser_context: AVCodecParserContext,

    yuv420p_decoder: YUV420PDecoder
}

impl H264Decoder {
    pub fn new(width: usize, height: usize) -> Self {
        let frame_buffer_size = width * height * 3;

        let decoder = AVCodec::find_decoder_by_name(cstr!("h264")).unwrap();

        H264Decoder {
            decoded_frame_buffer: vec![0 as u8; frame_buffer_size],
            decode_context: {
                let mut decode_context = AVCodecContext::new(&decoder);
                decode_context.open(None).unwrap();

                decode_context
            },

            parsed_offset: 0,
            parser_context: AVCodecParserContext::find(decoder.id).unwrap(),

            yuv420p_decoder: YUV420PDecoder::new(width, height)
        }
    }

    fn decoded_yuv_to_rgb(
        &mut self,
        y_frame_buffer: &[u8],
        u_frame_buffer: &[u8],
        v_frame_buffer: &[u8]
    ) {
        // TODO: Remove fill
        self.decoded_frame_buffer.fill(0);

        let mut yuv420p_frame_buffer = Vec::new();
        yuv420p_frame_buffer.extend_from_slice(y_frame_buffer);
        yuv420p_frame_buffer.extend_from_slice(u_frame_buffer);
        yuv420p_frame_buffer.extend_from_slice(v_frame_buffer);

        self.yuv420p_decoder.decode(&yuv420p_frame_buffer).unwrap();

        self.decoded_frame_buffer.copy_from_slice(&self.yuv420p_decoder.get_decoded_frame());
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
                        debug!("Error on send packet: {}", e);
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

                    let data = avframe.data;
                    let linesize = avframe.linesize;
                    // let width = avframe.width as usize;
                    let height = avframe.height as usize;

                    let linesize_y = linesize[0] as usize;
                    let linesize_cb = linesize[1] as usize;
                    let linesize_cr = linesize[2] as usize;
                    let y_data =
                        unsafe { std::slice::from_raw_parts_mut(data[0], height * linesize_y) };
                    let cb_data = unsafe {
                        std::slice::from_raw_parts_mut(data[1], height / 2 * linesize_cb)
                    };
                    let cr_data = unsafe {
                        std::slice::from_raw_parts_mut(data[2], height / 2 * linesize_cr)
                    };

                    self.decoded_yuv_to_rgb(y_data, cb_data, cr_data);
                    // self.decoded_frame_buffer.copy_from_slice(yuv_frame_buffer);
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
