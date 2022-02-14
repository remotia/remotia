use super::ReceivedFrameStats;

pub mod csv;
pub mod console;

pub trait ReceptionRoundLogger {
    fn log(&mut self, profiled_frames: &Vec<ReceivedFrameStats>);
}