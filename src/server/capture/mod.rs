pub mod scrap;

pub trait FrameCapturer {
    fn capture(&mut self) -> Result<&[u8], std::io::Error>;

    fn width(&self) -> usize;
    fn height(&self) -> usize;
}