#![allow(dead_code)]

pub mod udp;
pub mod tcp;

pub trait FrameSender {
    fn send_frame(&mut self, frame_buffer: &[u8]);
}