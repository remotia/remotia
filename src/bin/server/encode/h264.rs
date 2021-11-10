use rsmpeg::{avcodec::{AVCodec, AVCodecContext}, avutil::AVFrame, error::RsmpegError, ffi};

use cstr::cstr;

use super::Encoder;

pub struct H264Encoder {
    encoded_frame_buffer: Vec<u8>,
    encode_context: AVCodecContext,

    output_context: ffi::AVFormatContext,

    width: i32,
    height: i32,
}

impl H264Encoder {
    pub fn new(frame_buffer_size: usize, width: i32, height: i32) -> Self {
        H264Encoder {
            encoded_frame_buffer: vec![0 as u8; frame_buffer_size],
            width: width,
            height: height,

            encode_context: {
                let encoder = AVCodec::find_encoder_by_name(cstr!("libx264")).unwrap();
                let mut encode_context = AVCodecContext::new(&encoder);
                encode_context.set_bit_rate(400000);
                encode_context.set_width(width);
                encode_context.set_height(height);
                encode_context.set_time_base(ffi::AVRational{ num: 1, den: 60 });
                encode_context.set_framerate(ffi::AVRational{ num: 60, den: 1 });
                encode_context.set_gop_size(10);
                encode_context.set_max_b_frames(1);
                encode_context.set_pix_fmt(rsmpeg::ffi::AVPixelFormat_AV_PIX_FMT_YUV444P10);
                encode_context.open(None).unwrap();

                encode_context
            },

            output_context: unsafe {
                *ffi::avformat_alloc_context()
            },
        }
    }

    fn create_avframe(&self, frame_buffer: &[u8]) -> AVFrame {
        let mut avframe = AVFrame::new();
        avframe.set_format(rsmpeg::ffi::AVPixelFormat_AV_PIX_FMT_RGB24);
        avframe.set_width(self.width);
        avframe.set_height(self.height);
        avframe.alloc_buffer().unwrap();

        avframe.linesize_mut()[0] = self.width;
        avframe.linesize_mut()[1] = self.width;
        avframe.linesize_mut()[2] = self.width;

        let data = avframe.data[0];
        let linesize = avframe.linesize[0] as usize;
        let width = self.width as usize;
        let height = self.height as usize;
        let rgb_data = unsafe { std::slice::from_raw_parts_mut(data, height * linesize * 3) };
        for y in 0..height {
            for x in 0..width {
                rgb_data[y * linesize + x * 3 + 0] = frame_buffer[(y * width + x) * 3 + 0];
                rgb_data[y * linesize + x * 3 + 1] = frame_buffer[(y * width + x) * 3 + 1];
                rgb_data[y * linesize + x * 3 + 2] = frame_buffer[(y * width + x) * 3 + 2];
            }
        }

        avframe
    }
}

impl Encoder for H264Encoder {
    fn encode(&mut self, frame_buffer: &[u8]) -> usize {
        let encoded_frame_length = frame_buffer.len();

        let avframe = self.create_avframe(frame_buffer);

        self.encode_context.send_frame(Some(&avframe)).unwrap();

        loop {
            let mut packet = match self.encode_context.receive_packet() {
                Ok(packet) => packet,
                Err(RsmpegError::EncoderDrainError) | Err(RsmpegError::EncoderFlushedError) => {
                    break
                }
                Err(e) => panic!("{:?}", e),
            };

            let packet_ref = &mut packet;
            let output_context_ref: &mut ffi::AVFormatContext = &mut self.output_context;

            unsafe { ffi::av_write_frame(output_context_ref, packet_ref.as_mut_ptr()); }
        }

        encoded_frame_length
    }
    fn get_encoded_frame(&self) -> &[u8] {
        self.encoded_frame_buffer.as_slice()
    }
}
