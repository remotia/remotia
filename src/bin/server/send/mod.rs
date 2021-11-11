pub mod udp;

pub trait FrameSender {
    fn send_frame(&self, frame_buffer: &[u8]);
}