use super::{TransmittedFrameStats};

pub mod console;
pub mod csv;

pub trait TransmissionRoundLogger {
    fn log(&mut self, profiled_frames: &Vec<TransmittedFrameStats>);
}

