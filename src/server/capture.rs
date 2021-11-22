use scrap::{Capturer, Frame};

use std::io::ErrorKind::WouldBlock;

pub fn capture_frame<'a>(capturer: &'a mut Capturer) -> Result<Frame, std::io::Error> {
    match capturer.frame() {
        Ok(buffer) => return Ok(buffer),
        Err(error) => {
            if error.kind() == WouldBlock {
                return Err(error);
            } else {
                panic!("Error: {}", error);
            }
        }
    };
}