use log::debug;
use rsmpeg::{avcodec::AVCodecContext, avutil::AVFrame, error::RsmpegError};

use crate::server::utils::bgr2yuv::raster;

pub struct YUV420PAVFrameBuilder {
    frame_count: i64,
    y_pixels: Vec<u8>,
    u_pixels: Vec<u8>,
    v_pixels: Vec<u8>,
}

impl YUV420PAVFrameBuilder {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            frame_count: 0,
            y_pixels: vec![0; width * height],
            u_pixels: vec![0; (width * height) / 4],
            v_pixels: vec![0; (width * height) / 4],
        }
    }

    pub fn create_avframe(
        &mut self,
        encode_context: &mut AVCodecContext,
        frame_buffer: &[u8],
        key_frame: bool
    ) -> AVFrame {
        let mut avframe = AVFrame::new();
        avframe.set_format(encode_context.pix_fmt);
        avframe.set_width(encode_context.width);
        avframe.set_height(encode_context.height);
        avframe.set_pts(self.frame_count);
        if key_frame {
            avframe.set_pict_type(1);
        }
        avframe.alloc_buffer().unwrap();

        let data = avframe.data;
        let linesize = avframe.linesize;
        // let width = encode_context.width as usize;
        let height = encode_context.height as usize;

        let linesize_y = linesize[0] as usize;
        let linesize_cb = linesize[1] as usize;
        let linesize_cr = linesize[2] as usize;
        let y_data = unsafe { std::slice::from_raw_parts_mut(data[0], height * linesize_y) };
        let cb_data = unsafe { std::slice::from_raw_parts_mut(data[1], height / 2 * linesize_cb) };
        let cr_data = unsafe { std::slice::from_raw_parts_mut(data[2], height / 2 * linesize_cr) };

        self.u_pixels.fill(0);
        self.v_pixels.fill(0);

        raster::bgra_to_yuv_separate(
            frame_buffer,
            &mut self.y_pixels,
            &mut self.u_pixels,
            &mut self.v_pixels,
        );

        y_data.copy_from_slice(&self.y_pixels);
        cb_data.copy_from_slice(&self.u_pixels);
        cr_data.copy_from_slice(&self.v_pixels);

        debug!("Created avframe #{}", avframe.pts);

        self.frame_count += 1;

        avframe
    }
}
