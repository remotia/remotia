use log::{debug};
use rsmpeg::{avcodec::AVCodecContext, avutil::AVFrame};

pub struct YUV420PAVFrameBuilder {
    frame_count: i64,
}

impl YUV420PAVFrameBuilder {
    pub fn new() -> Self {
        Self { frame_count: 0 }
    }

    pub fn create_avframe(
        &mut self,
        encode_context: &mut AVCodecContext,
        y_channel_buffer: &[u8],
        cb_channel_buffer: &[u8],
        cr_channel_buffer: &[u8],
        force_key_frame: bool,
    ) -> AVFrame {
        let mut avframe = AVFrame::new();
        avframe.set_format(encode_context.pix_fmt);
        avframe.set_width(encode_context.width);
        avframe.set_height(encode_context.height);
        avframe.set_pts(self.frame_count);

        /*let avframe = unsafe {
            let raw_avframe = avframe.into_raw().as_ptr();
            (*raw_avframe).opaque = 100;
            AVFrame::from_raw(NonNull::new(raw_avframe).unwrap())
        };*/

        if force_key_frame {
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
        let cb_data =
            unsafe { std::slice::from_raw_parts_mut(data[1], height / 2 * linesize_cb) };
        let cr_data =
            unsafe { std::slice::from_raw_parts_mut(data[2], height / 2 * linesize_cr) };

        y_data.copy_from_slice(y_channel_buffer);
        cb_data.copy_from_slice(cb_channel_buffer);
        cr_data.copy_from_slice(cr_channel_buffer);

        debug!("Created avframe #{}", avframe.pts);

        self.frame_count += 1;

        avframe
    }
}
