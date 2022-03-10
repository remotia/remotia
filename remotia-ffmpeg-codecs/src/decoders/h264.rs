use std::collections::VecDeque;

use log::debug;
use rsmpeg::{
    avcodec::{AVCodec, AVCodecContext, AVCodecParserContext, AVPacket},
    error::RsmpegError,
    ffi, UnsafeDerefMut,
};

use cstr::cstr;

use remotia::{
    error::DropReason,
    traits::FrameProcessor, types::FrameData,
};

use super::utils::yuv2bgr::raster;
use async_trait::async_trait;

pub struct H264Decoder {
    decode_context: AVCodecContext,

    timestamps_queue: VecDeque<u128>,

    parser_context: AVCodecParserContext,
}

// TODO: Fix all those unsafe impl
unsafe impl Send for H264Decoder {}

impl H264Decoder {
    pub fn new() -> Self {
        let decoder = AVCodec::find_decoder_by_name(cstr!("h264")).unwrap();

        H264Decoder {
            decode_context: {
                let mut decode_context = AVCodecContext::new(&decoder);
                decode_context.open(None).unwrap();

                decode_context
            },

            parser_context: AVCodecParserContext::find(decoder.id).unwrap(),
            timestamps_queue: VecDeque::new(),
        }
    }

    fn decoded_yuv_to_bgra(
        &mut self,
        y_frame_buffer: &[u8],
        u_frame_buffer: &[u8],
        v_frame_buffer: &[u8],
        output_buffer: &mut [u8],
    ) {
        raster::yuv_to_bgra_separate(
            y_frame_buffer,
            u_frame_buffer,
            v_frame_buffer,
            output_buffer,
        );
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
        self.decoded_yuv_to_bgra(y_data, cb_data, cr_data, output_buffer);
    }

    fn parse_packets(&mut self, input_buffer: &[u8]) -> Option<DropReason> {
        let mut packet = AVPacket::new();
        let mut parsed_offset = 0;

        debug!(
            "Parsing packets (input buffer size: {})...",
            input_buffer.len()
        );

        while parsed_offset < input_buffer.len() {
            let (get_packet, offset) = {
                let this = &mut self.parser_context;

                let codec_context: &mut AVCodecContext = &mut self.decode_context;
                let packet: &mut AVPacket = &mut packet;
                let data: &[u8] = &input_buffer[parsed_offset..];
                let mut packet_data = packet.data;
                let mut packet_size = packet.size;

                let pts = 0x4000000000000000u64 as i64;
                let offset = unsafe {
                    ffi::av_parser_parse2(
                        this.as_mut_ptr(),
                        codec_context.as_mut_ptr(),
                        &mut packet_data,
                        &mut packet_size,
                        data.as_ptr(),
                        data.len() as i32,
                        pts,
                        pts,
                        0,
                    )
                };

                unsafe {
                    packet.deref_mut().data = packet_data;
                    packet.deref_mut().size = packet_size;
                }

                (packet.size != 0, offset as usize)
            };

            if get_packet {
                let result = self.decode_context.send_packet(Some(&packet));

                match result {
                    Ok(_) => (),
                    Err(e) => {
                        debug!("Error on send packet: {}", e);
                        return Some(DropReason::CodecError);
                    }
                }

                debug!("Decoded packet: {:?}", packet);

                packet = AVPacket::new();
            }

            parsed_offset += offset;
        }

        None
    }

    fn decode_to_buffer(
        &mut self,
        input_buffer: &[u8],
        output_buffer: &mut [u8],
    ) -> Result<i64, DropReason> {
        if let Some(error) = self.parse_packets(input_buffer) {
            return Err(error);
        }

        let avframe = match self.decode_context.receive_frame() {
            Ok(frame) => frame,
            Err(RsmpegError::DecoderDrainError) | Err(RsmpegError::DecoderFlushedError) => {
                return Err(DropReason::NoDecodedFrames);
            }
            Err(e) => panic!("{:?}", e),
        };

        let frame_id = avframe.reordered_opaque as i64;

        self.write_avframe(avframe, output_buffer);

        Ok(frame_id)
    }
}

impl Default for H264Decoder {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FrameProcessor for H264Decoder {
    async fn process(&mut self, mut frame_data: FrameData) -> Option<FrameData> {
        let mut encoded_frame_buffer = frame_data
            .extract_writable_buffer("encoded_frame_buffer")
            .unwrap();

        let empty_buffer_memory =
            encoded_frame_buffer.split_off(frame_data.get("encoded_size") as usize);

        let mut raw_frame_buffer = frame_data
            .extract_writable_buffer("raw_frame_buffer")
            .unwrap();

        // Enqueue timestamp, pasting it to the next decoded frame available
        // Useful to compensate codec delay, may not work when frames
        // are not decoded in order
        debug!("Pushing timestamp: {}", frame_data.get("capture_timestamp"));
        self.timestamps_queue
            .push_back(frame_data.get("capture_timestamp"));

        let decode_result = self.decode_to_buffer(&encoded_frame_buffer, &mut raw_frame_buffer);

        match decode_result {
            Ok(_) | Err(DropReason::CodecError) => {
                // Override capture timestamp, compensating eventual decoding delay
                debug!("Enqueued timestamps: {:?}", self.timestamps_queue);
                let capture_timestamp = self.timestamps_queue.pop_front().unwrap();
                debug!(
                    "Popping timestamp: {}, current frame timestamp: {}",
                    capture_timestamp,
                    frame_data.get("capture_timestamp")
                );
                frame_data.set("capture_timestamp", capture_timestamp);
            }
            Err(_) => (),
        }

        encoded_frame_buffer.unsplit(empty_buffer_memory);

        frame_data.insert_writable_buffer("encoded_frame_buffer", encoded_frame_buffer);
        frame_data.insert_writable_buffer("raw_frame_buffer", raw_frame_buffer);

        if let Err(drop_reason) = decode_result {
            frame_data.set_drop_reason(Some(drop_reason));
        }

        Some(frame_data)
    }
}
