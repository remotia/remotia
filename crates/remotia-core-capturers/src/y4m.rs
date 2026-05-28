use std::fs::File;

use async_trait::async_trait;
use log::debug;
use remotia_buffer_utils::BytesMut;
use remotia_core::pipeline::PipelineHandle;
use remotia_core::traits::{BorrowMutFrameProperties, FrameProcessor, FrameProperties, PullableFrameProperties};
use remotia_buffer_utils::BufMut;
use y4m::Decoder;

/// A Y4M frame capturer that writes raw Y+U+V planes into a pre-allocated buffer.
///
/// Requires the frame data to already contain a buffer at `buffer_key` with sufficient capacity
/// for the concatenated Y, U, and V planes. Returns `None` when the stream is exhausted.
pub struct Y4MFrameCapturer<K> {
    stream: Decoder<File>,
    buffer_key: K,
}

impl<K> Y4MFrameCapturer<K> {
    /// Creates a new capturer that reads Y4M frames from the file at `path`.
    pub fn new(buffer_key: K, path: &str) -> Self {
        Self {
            stream: y4m::decode(File::open(path).unwrap()).unwrap(),
            buffer_key,
        }
    }
}

#[async_trait]
impl<F, K> FrameProcessor<F> for Y4MFrameCapturer<K>
where
    K: Send,
    F: BorrowMutFrameProperties<K, BytesMut> + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        let frame = self.stream.read_frame();
        if frame.is_err() {
            debug!("No more frames to extract");
            return None;
        }

        let frame = frame.unwrap();

        let buffer = frame_data.get_mut_ref(&self.buffer_key).unwrap();
        buffer.put(frame.get_y_plane());
        buffer.put(frame.get_u_plane());
        buffer.put(frame.get_v_plane());

        Some(frame_data)
    }
}

/// A Y4M frame capturer that converts YUV420p frames to RGBA and pushes them into frame data.
///
/// Unlike [`Y4MFrameCapturer`], this processor:
/// - Converts YUV420p to RGBA on the fly
/// - Allocates and pushes the RGBA buffer via [`PullableFrameProperties`]
/// - Tracks frame IDs via [`FrameProperties`]
/// - Emits an EOF-marked frame before exhaustion, allowing downstream processors to flush
/// - Signals the pipeline to shut down after EOF
///
/// # Type parameters
/// - `BK`: Buffer key type used to store the RGBA frame data
/// - `SK`: Stat key type used for EOF and frame-ID metadata
pub struct Y4MRGBAFrameCapturer<BK, SK> {
    stream: Decoder<File>,
    buffer_key: BK,
    eof_stat_key: SK,
    frame_id_stat_key: SK,
    width: usize,
    height: usize,
    frame_id: u128,
    max_frames: Option<u64>,
    eof_emitted: bool,
    pipeline_handle: PipelineHandle,
}

impl<BK, SK> Y4MRGBAFrameCapturer<BK, SK> {
    /// Creates a new RGBA capturer from an existing Y4M decoder.
    ///
    /// The caller is responsible for opening and parsing the Y4M file before passing
    /// the decoder here. This is useful when the caller needs to inspect the stream
    /// dimensions (via [`Self::width`] / [`Self::height`]) before building the pipeline.
    pub fn from_decoder(
        stream: Decoder<File>,
        buffer_key: BK,
        eof_stat_key: SK,
        frame_id_stat_key: SK,
        max_frames: Option<u64>,
        pipeline_handle: PipelineHandle,
    ) -> Self {
        let width = stream.get_width();
        let height = stream.get_height();
        Self {
            stream,
            buffer_key,
            eof_stat_key,
            frame_id_stat_key,
            width,
            height,
            frame_id: 0,
            max_frames,
            eof_emitted: false,
            pipeline_handle,
        }
    }

    /// Returns the width of the Y4M video stream.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Returns the height of the Y4M video stream.
    pub fn height(&self) -> usize {
        self.height
    }
}

#[async_trait]
impl<F, BK, SK> FrameProcessor<F> for Y4MRGBAFrameCapturer<BK, SK>
where
    BK: Copy + Send,
    SK: Copy + Send,
    F: PullableFrameProperties<BK, BytesMut> + FrameProperties<SK, u128> + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        if self.eof_emitted {
            self.pipeline_handle.request_shutdown();
            return None;
        }

        if let Some(max) = self.max_frames {
            if self.frame_id >= max as u128 {
                log::info!("Reached frame limit ({})", max);
                frame_data.set(self.eof_stat_key, 1);
                self.eof_emitted = true;
                return Some(frame_data);
            }
        }

        match self.stream.read_frame() {
            Ok(frame) => {
                let y = frame.get_y_plane();
                let u = frame.get_u_plane();
                let v = frame.get_v_plane();

                let mut rgba = BytesMut::with_capacity(self.width * self.height * 4);
                yuv420_to_rgba(y, u, v, self.width, self.height, &mut rgba);

                frame_data.push(self.buffer_key, rgba);
                frame_data.set(self.frame_id_stat_key, self.frame_id);
                self.frame_id += 1;

                Some(frame_data)
            }
            Err(_) => {
                debug!("Y4M EOF after {} frames, sending EOF frame", self.frame_id);
                frame_data.set(self.eof_stat_key, 1);
                self.eof_emitted = true;
                Some(frame_data)
            }
        }
    }
}

/// Converts a YUV420p frame to RGBA, writing the result into `out`.
///
/// Uses the full BT.601 conversion matrix with proper chroma siting
/// (UV planes are subsampled 2x2 relative to Y).
pub fn yuv420_to_rgba(
    y_plane: &[u8],
    u_plane: &[u8],
    v_plane: &[u8],
    width: usize,
    height: usize,
    out: &mut BytesMut,
) {
    let uv_width = width / 2;
    for row in 0..height {
        for col in 0..width {
            let y_idx = row * width + col;
            let uv_row = row / 2;
            let uv_col = col / 2;
            let uv_idx = uv_row * uv_width + uv_col;

            let y = y_plane[y_idx] as f32;
            let u = u_plane[uv_idx] as f32 - 128.0;
            let v = v_plane[uv_idx] as f32 - 128.0;

            let r = (y + 1.402 * v).clamp(0.0, 255.0) as u8;
            let g = (y - 0.344 * u - 0.714 * v).clamp(0.0, 255.0) as u8;
            let b = (y + 1.772 * u).clamp(0.0, 255.0) as u8;

            out.extend_from_slice(&[r, g, b, 255]);
        }
    }
}
