use std::slice;

use log::debug;
use rsmpeg::{
    avcodec::{AVCodec, AVCodecContext, AVCodecParserContext, AVPacket},
    error::RsmpegError,
};

use cstr::cstr;

use crate::{client::error::ClientError, common::feedback::FeedbackMessage};

use super::{utils::yuv2bgr::raster, Decoder};

pub struct H265Decoder {
    decode_context: AVCodecContext,

    parser_context: AVCodecParserContext,
}

// TODO: Fix all those unsafe impl
unsafe impl Send for H265Decoder {}

impl H265Decoder {
    pub fn new() -> Self {
        let decoder = AVCodec::find_decoder_by_name(cstr!("hevc")).unwrap();

        H265Decoder {
            decode_context: {
                let mut decode_context = AVCodecContext::new(&decoder);
                decode_context.open(None).unwrap();

                decode_context
            },

            parser_context: AVCodecParserContext::find(decoder.id).unwrap(),
        }
    }

    fn decoded_yuv_to_rgb(
        &mut self,
        y_frame_buffer: &[u8],
        u_frame_buffer: &[u8],
        v_frame_buffer: &[u8],
        output_buffer: &mut [u8],
    ) {
        // TODO: Remove fill
        let mut yuv420p_frame_buffer = Vec::new();
        yuv420p_frame_buffer.extend_from_slice(y_frame_buffer);
        yuv420p_frame_buffer.extend_from_slice(u_frame_buffer);
        yuv420p_frame_buffer.extend_from_slice(v_frame_buffer);

        raster::yuv_to_bgr(&yuv420p_frame_buffer, output_buffer);
    }

    fn write_avframe(&mut self, avframe: rsmpeg::avutil::AVFrame, output_buffer: &mut [u8]) {
        let data = avframe.data;
        let linesize = avframe.linesize;
        let height = avframe.height as usize;
        let linesize_y = linesize[0] as usize;
        let linesize_cb = linesize[1] as usize;
        let linesize_cr = linesize[2] as usize;
        let y_data = unsafe { std::slice::from_raw_parts_mut(data[0], height * linesize_y) };
        let cb_data = unsafe { std::slice::from_raw_parts_mut(data[1], height / 2 * linesize_cb) };
        let cr_data = unsafe { std::slice::from_raw_parts_mut(data[2], height / 2 * linesize_cr) };
        self.decoded_yuv_to_rgb(y_data, cb_data, cr_data, output_buffer);
    }

    fn parse_packets(&mut self, input_buffer: &[u8]) -> Option<ClientError> {
        let mut packet = AVPacket::new();
        let mut parsed_offset = 0;
        while parsed_offset < input_buffer.len() {
            let (get_packet, offset) = self
                .parser_context
                .parse_packet(
                    &mut self.decode_context,
                    &mut packet,
                    &input_buffer[parsed_offset..],
                )
                .unwrap();

            if get_packet {
                let result = self.decode_context.send_packet(Some(&packet));

                match result {
                    Ok(_) => (),
                    Err(e) => {
                        debug!("Error on send packet: {}", e);
                        return Some(ClientError::FFMpegSendPacketError);
                    }
                }

                packet = AVPacket::new();
            }

            parsed_offset += offset;
        }

        None
    }
}

impl Decoder for H265Decoder {
    fn decode(
        &mut self,
        input_buffer: &[u8],
        output_buffer: &mut [u8],
    ) -> Result<usize, ClientError> {
        if let Some(error) = self.parse_packets(input_buffer) {
            return Err(error);
        }

        let avframe = match self.decode_context.receive_frame() {
            Ok(frame) => frame,
            Err(RsmpegError::DecoderDrainError) | Err(RsmpegError::DecoderFlushedError) => {
                return Err(ClientError::NoDecodedFrames);
            }
            Err(e) => panic!("{:?}", e),
        };

        self.write_avframe(avframe, output_buffer);

        Ok(output_buffer.len())
    }

    fn handle_feedback(&mut self, message: FeedbackMessage) {
        debug!("Feedback message: {:?}", message);
    }
}

