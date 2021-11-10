use std::ffi::{CStr, CString};
use std::error::Error;
use rsmpeg::avformat::AVFormatContextInput;

fn dump_av_info(path: &CStr) -> Result<(), Box<dyn Error>> {
    let mut input_format_context = AVFormatContextInput::open(path)?;
    input_format_context.dump(0, path)?;
    Ok(())
}

fn main() {
    dump_av_info(&CString::new("./test.jpg").unwrap()).unwrap();
}
