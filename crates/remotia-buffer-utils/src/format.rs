//! Pixel-format conversion between packed `RGB`/`BGR`/`RGBA`/`BGRA` frame buffers.
//!
//! [`FormatConverter`] is the pure conversion engine: it swaps the red/blue
//! channels when the ends disagree on the order, drops the alpha channel on
//! `*_A -> *` conversions and inserts a fully opaque alpha (`255`) on
//! `* -> *_A` expansions, preserving a carried alpha otherwise.
//!
//! [`BufferFormatter`] exposes the engine as a pipeline stage, built fluently
//! as `BufferFormatter::source(key, format).dest(key, format)` and appended
//! like any other [`FrameProcessor`](remotia_core::traits::FrameProcessor).
//! A stage with equal source and destination keys converts in place (channel
//! swaps only); otherwise the source buffer is converted into the destination
//! buffer, which is resized to fit. Errored frames and missing/invalid source
//! buffers are skipped with a log line, leaving the destination untouched —
//! error policy stays with the surrounding stages (drop guards, [`FillBlack`]).
//!
//! [`FillBlack`] paints a buffer opaque black: it sizes the destination to its
//! configured byte length and fills it on errored frames, and zero-fills an
//! empty destination on healthy frames that produced no pixels (so downstream
//! length checks keep passing).
//!
//! ```rust
//! use remotia_buffer_utils::{FormatConverter, PixelFormat};
//!
//! let converter = FormatConverter::new(PixelFormat::Bgra, PixelFormat::Rgba);
//! let rgba = converter.format(&[10, 20, 30, 255]).unwrap();
//! assert_eq!(rgba, vec![30, 20, 10, 255]);
//! ```

use std::fmt;
use std::marker::PhantomData;

use async_trait::async_trait;
use remotia_core::traits::{BorrowMutFrameProperties, FrameError, FrameProcessor};

use crate::BytesMut;

/// Packed pixel layout of a frame buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PixelFormat {
    /// Three bytes per pixel: red, green, blue.
    Rgb,
    /// Three bytes per pixel: blue, green, red.
    Bgr,
    /// Four bytes per pixel: red, green, blue, alpha.
    Rgba,
    /// Four bytes per pixel: blue, green, red, alpha.
    Bgra,
}

impl PixelFormat {
    /// Bytes of a single pixel in this format.
    pub const fn bytes_per_pixel(self) -> usize {
        match self {
            Self::Rgb | Self::Bgr => 3,
            Self::Rgba | Self::Bgra => 4,
        }
    }

    /// Whether pixels carry an alpha channel.
    pub const fn has_alpha(self) -> bool {
        match self {
            Self::Rgb | Self::Bgr => false,
            Self::Rgba | Self::Bgra => true,
        }
    }

    /// Whether red comes before blue in memory (`Rgb`/`Rgba`).
    const fn is_red_first(self) -> bool {
        matches!(self, Self::Rgb | Self::Rgba)
    }

    /// Fills `dst` (packed in `self`) with opaque black.
    ///
    /// The colour channels are zeroed and the alpha channel — when present —
    /// is set to `255`. Fails when `dst` does not hold a whole number of
    /// pixels.
    pub fn fill_black(self, dst: &mut [u8]) -> Result<(), BufferFormatError> {
        let bpp = self.bytes_per_pixel();
        if !dst.len().is_multiple_of(bpp) {
            return Err(BufferFormatError::invalid_source_length(dst.len(), bpp));
        }
        if self.has_alpha() {
            for pixel in dst.chunks_exact_mut(bpp) {
                pixel[0] = 0;
                pixel[1] = 0;
                pixel[2] = 0;
                pixel[3] = 0xFF;
            }
        } else {
            dst.fill(0);
        }
        Ok(())
    }
}

/// Conversion failure between two packed pixel buffers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BufferFormatError {
    /// The source slice does not hold a whole number of source pixels.
    InvalidSourceLength {
        /// Byte length that was passed in.
        len: usize,
        /// Bytes per source pixel.
        bytes_per_pixel: usize,
    },
    /// The destination slice does not fit exactly the converted pixels.
    MismatchedDestLength {
        /// Byte length that was passed in.
        len: usize,
        /// Byte length the conversion needs.
        expected: usize,
    },
    /// An in-place conversion needs equal source/destination pixel sizes.
    InPlaceDifferentPixelSizes {
        /// Bytes per source pixel.
        source_bytes_per_pixel: usize,
        /// Bytes per destination pixel.
        dest_bytes_per_pixel: usize,
    },
}

impl BufferFormatError {
    fn invalid_source_length(len: usize, bytes_per_pixel: usize) -> Self {
        Self::InvalidSourceLength {
            len,
            bytes_per_pixel,
        }
    }

    fn mismatched_dest_length(len: usize, expected: usize) -> Self {
        Self::MismatchedDestLength { len, expected }
    }
}

impl fmt::Display for BufferFormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSourceLength {
                len,
                bytes_per_pixel,
            } => write!(
                f,
                "source holds {len} bytes, which is not a whole number of {bytes_per_pixel}-byte pixels"
            ),
            Self::MismatchedDestLength { len, expected } => write!(
                f,
                "destination holds {len} bytes but the conversion needs {expected} bytes"
            ),
            Self::InPlaceDifferentPixelSizes {
                source_bytes_per_pixel,
                dest_bytes_per_pixel,
            } => write!(
                f,
                "in-place conversion needs equal pixel sizes, got {source_bytes_per_pixel} -> {dest_bytes_per_pixel} bytes per pixel"
            ),
        }
    }
}

impl std::error::Error for BufferFormatError {}

/// Converts packed frame buffers from one [`PixelFormat`] to another.
///
/// Same-format conversions are plain copies. All methods validate the buffer
/// lengths and fail with [`BufferFormatError`] instead of truncating or
/// overrunning.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FormatConverter {
    source_format: PixelFormat,
    dest_format: PixelFormat,
}

impl FormatConverter {
    /// Builds a converter from `source_format` to `dest_format`.
    pub fn new(source_format: PixelFormat, dest_format: PixelFormat) -> Self {
        Self {
            source_format,
            dest_format,
        }
    }

    /// The input pixel layout.
    pub fn source_format(self) -> PixelFormat {
        self.source_format
    }

    /// The output pixel layout.
    pub fn dest_format(self) -> PixelFormat {
        self.dest_format
    }

    /// Number of pixels `src` holds, checking it is whole source pixels.
    fn pixel_count(self, src_len: usize) -> Result<usize, BufferFormatError> {
        let bpp = self.source_format.bytes_per_pixel();
        if !src_len.is_multiple_of(bpp) {
            return Err(BufferFormatError::invalid_source_length(src_len, bpp));
        }
        Ok(src_len / bpp)
    }

    /// Byte length `dst` must have to hold the conversion of `src`.
    pub fn dest_len(self, src_len: usize) -> Result<usize, BufferFormatError> {
        Ok(self.pixel_count(src_len)? * self.dest_format.bytes_per_pixel())
    }

    /// Converts `src` into `dst`, which must already have the exact length
    /// [`Self::dest_len`] reports.
    pub fn convert(self, src: &[u8], dst: &mut [u8]) -> Result<(), BufferFormatError> {
        let pixels = self.pixel_count(src.len())?;
        let expected = pixels * self.dest_format.bytes_per_pixel();
        if dst.len() != expected {
            return Err(BufferFormatError::mismatched_dest_length(
                dst.len(),
                expected,
            ));
        }

        let src_bpp = self.source_format.bytes_per_pixel();
        let dst_bpp = self.dest_format.bytes_per_pixel();
        let src_red_first = self.source_format.is_red_first();
        let dst_red_first = self.dest_format.is_red_first();
        let src_has_alpha = self.source_format.has_alpha();
        let dst_has_alpha = self.dest_format.has_alpha();

        for (src_pixel, dst_pixel) in src.chunks_exact(src_bpp).zip(dst.chunks_exact_mut(dst_bpp)) {
            // Normalise to R, G, B (+A when carried).
            let (r, g, b) = if src_red_first {
                (src_pixel[0], src_pixel[1], src_pixel[2])
            } else {
                (src_pixel[2], src_pixel[1], src_pixel[0])
            };

            if dst_red_first {
                dst_pixel[0] = r;
                dst_pixel[1] = g;
                dst_pixel[2] = b;
            } else {
                dst_pixel[0] = b;
                dst_pixel[1] = g;
                dst_pixel[2] = r;
            }

            if dst_has_alpha {
                dst_pixel[3] = if src_has_alpha { src_pixel[3] } else { 0xFF };
            }
        }

        Ok(())
    }

    /// Allocating variant of [`Self::convert`].
    pub fn format(self, src: &[u8]) -> Result<Vec<u8>, BufferFormatError> {
        let mut dst = vec![0u8; self.dest_len(src.len())?];
        self.convert(src, &mut dst)?;
        Ok(dst)
    }

    /// Converts `buf` in place.
    ///
    /// Only same-size conversions qualify (`RGB <-> BGR`, `RGBA <-> BGRA`,
    /// identity): anything else fails with
    /// [`BufferFormatError::InPlaceDifferentPixelSizes`]. Identity is a
    /// no-op, channel swaps exchange bytes 0 and 2 of every pixel.
    pub fn convert_in_place(self, buf: &mut [u8]) -> Result<(), BufferFormatError> {
        let src_bpp = self.source_format.bytes_per_pixel();
        let dst_bpp = self.dest_format.bytes_per_pixel();
        if src_bpp != dst_bpp {
            return Err(BufferFormatError::InPlaceDifferentPixelSizes {
                source_bytes_per_pixel: src_bpp,
                dest_bytes_per_pixel: dst_bpp,
            });
        }
        if !buf.len().is_multiple_of(src_bpp) {
            return Err(BufferFormatError::invalid_source_length(buf.len(), src_bpp));
        }
        if self.source_format.is_red_first() != self.dest_format.is_red_first() {
            for pixel in buf.chunks_exact_mut(src_bpp) {
                pixel.swap(0, 2);
            }
        }
        Ok(())
    }

    /// Fills `dst` (packed in the destination format) with opaque black.
    ///
    /// Shorthand for `self.dest_format().fill_black(dst)`.
    pub fn fill_black(self, dst: &mut [u8]) -> Result<(), BufferFormatError> {
        self.dest_format.fill_black(dst)
    }
}

/// Half-built [`BufferFormatter`]: the source buffer is known, the
/// destination is still missing. Returned by [`BufferFormatter::source`];
/// finish it with [`Self::dest`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BufferFormatterSource<K> {
    key: K,
    format: PixelFormat,
}

impl<K> BufferFormatterSource<K> {
    /// Sets the destination buffer and yields the pipeline stage.
    ///
    /// The error type `E` is inferred from the frame the stage is appended
    /// for (e.g. `Component::<HarnessFrame>::append` fixes `E` to that
    /// frame's error type); annotate it only when building a stage outside a
    /// pipeline, as `.dest::<MyError>(key, format)`.
    pub fn dest<E>(self, key: K, format: PixelFormat) -> BufferFormatter<K, E> {
        BufferFormatter {
            source_key: self.key,
            source_format: self.format,
            dest_key: key,
            dest_format: format,
            _error: PhantomData,
        }
    }
}

/// Converts one pooled frame buffer from one [`PixelFormat`] to another as a
/// pipeline stage.
///
/// Built as `BufferFormatter::source(key, format).dest(key, format)` and
/// appended to a [`Component`](remotia_core::pipeline::component::Component)
/// like any other processor. Equal source and destination keys convert in
/// place (channel swaps only); otherwise the source is converted into the
/// destination buffer, which is resized to fit.
///
/// Errored frames pass through untouched, and a missing source/destination
/// buffer or an invalid source length only logs and passes the frame on,
/// leaving the destination untouched: the stage never drops a frame, so it
/// needs no fused redemption. Downstream drop guards observe the empty
/// destination exactly as if the conversion had never run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BufferFormatter<K, E> {
    source_key: K,
    source_format: PixelFormat,
    dest_key: K,
    dest_format: PixelFormat,
    _error: PhantomData<E>,
}

impl BufferFormatter<(), ()> {
    /// Starts building a conversion stage from the `key` buffer packed in
    /// `format`; finish it with
    /// [`BufferFormatterSource::dest`](BufferFormatterSource::dest).
    pub fn source<K>(key: K, format: PixelFormat) -> BufferFormatterSource<K> {
        BufferFormatterSource { key, format }
    }
}

#[async_trait]
impl<F, K, E> FrameProcessor<F> for BufferFormatter<K, E>
where
    F: BorrowMutFrameProperties<K, BytesMut> + FrameError<E> + Send + 'static,
    K: Copy + PartialEq + Send + fmt::Debug,
    E: Send,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        if frame_data.get_error().is_some() {
            // Error policy belongs to the surrounding stages: drop guards
            // discard flagged frames, FillBlack paints them.
            return Some(frame_data);
        }

        let converter = FormatConverter::new(self.source_format, self.dest_format);

        if self.source_key == self.dest_key {
            let Some(buffer) = frame_data.get_mut_ref(&self.source_key) else {
                log::debug!(
                    "BufferFormatter: no {:?} buffer, skipping {:?} -> {:?} conversion",
                    self.source_key,
                    self.source_format,
                    self.dest_format,
                );
                return Some(frame_data);
            };
            if let Err(error) = converter.convert_in_place(buffer) {
                log::warn!(
                    "BufferFormatter: cannot convert {:?} buffer {:?} -> {:?}: {error}",
                    self.source_key,
                    self.source_format,
                    self.dest_format,
                );
            }
            return Some(frame_data);
        }

        // The source and destination both borrow `frame_data` mutably, so the
        // source is copied out before the destination is touched.
        let Some(source) = frame_data
            .get_mut_ref(&self.source_key)
            .map(|buffer| buffer.to_vec())
        else {
            log::debug!(
                "BufferFormatter: no {:?} buffer, skipping {:?} -> {:?} conversion",
                self.source_key,
                self.source_format,
                self.dest_format,
            );
            return Some(frame_data);
        };
        let Some(dest) = frame_data.get_mut_ref(&self.dest_key) else {
            log::warn!(
                "BufferFormatter: no {:?} buffer, skipping {:?} -> {:?} conversion",
                self.dest_key,
                self.source_format,
                self.dest_format,
            );
            return Some(frame_data);
        };

        let expected = match converter.dest_len(source.len()) {
            Ok(len) => len,
            Err(error) => {
                log::warn!(
                    "BufferFormatter: cannot convert {:?} buffer {:?} -> {:?}: {error}",
                    self.source_key,
                    self.source_format,
                    self.dest_format,
                );
                return Some(frame_data);
            }
        };
        dest.clear();
        dest.resize(expected, 0);
        if let Err(error) = converter.convert(&source, dest) {
            log::warn!(
                "BufferFormatter: cannot convert {:?} buffer {:?} -> {:?}: {error}",
                self.source_key,
                self.source_format,
                self.dest_format,
            );
        }
        Some(frame_data)
    }
}

/// Paints a pooled frame buffer opaque black as a pipeline stage.
///
/// On errored frames the `key` buffer is sized to `size_bytes` and filled
/// with opaque black in `format`, so a missing decoded frame shows a
/// deterministic image instead of stale pool contents. On healthy frames an
/// empty destination is zero-filled to `size_bytes` instead (a frame that
/// produced no pixels still passes downstream length checks); a destination
/// that already holds pixels is left untouched. A missing buffer only logs:
/// the stage never drops a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FillBlack<K, E> {
    key: K,
    format: PixelFormat,
    size_bytes: usize,
    _error: PhantomData<E>,
}

impl<K, E> FillBlack<K, E> {
    /// Builds a stage painting the `key` buffer (packed in `format`) over
    /// `size_bytes` bytes. The error type `E` is inferred from the frame the
    /// stage is appended for; annotate it (`FillBlack::<_, MyError>::new(..)`)
    /// only when building the stage outside a pipeline.
    pub fn new(key: K, format: PixelFormat, size_bytes: usize) -> Self {
        Self {
            key,
            format,
            size_bytes,
            _error: PhantomData,
        }
    }
}

#[async_trait]
impl<F, K, E> FrameProcessor<F> for FillBlack<K, E>
where
    F: BorrowMutFrameProperties<K, BytesMut> + FrameError<E> + Send + 'static,
    K: Copy + Send,
    E: Send,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        let errored = frame_data.get_error().is_some();
        let Some(dest) = frame_data.get_mut_ref(&self.key) else {
            log::debug!("FillBlack: no destination buffer, leaving the frame untouched");
            return Some(frame_data);
        };

        if errored {
            dest.clear();
            dest.resize(self.size_bytes, 0);
            if let Err(error) = self.format.fill_black(dest) {
                log::warn!("FillBlack: cannot paint the destination black: {error}");
            }
        } else if dest.is_empty() {
            dest.resize(self.size_bytes, 0);
        }
        Some(frame_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashMap;

    #[test]
    fn bytes_per_pixel_and_alpha_flags() {
        assert_eq!(PixelFormat::Rgb.bytes_per_pixel(), 3);
        assert_eq!(PixelFormat::Bgr.bytes_per_pixel(), 3);
        assert_eq!(PixelFormat::Rgba.bytes_per_pixel(), 4);
        assert_eq!(PixelFormat::Bgra.bytes_per_pixel(), 4);
        assert!(!PixelFormat::Rgb.has_alpha());
        assert!(!PixelFormat::Bgr.has_alpha());
        assert!(PixelFormat::Rgba.has_alpha());
        assert!(PixelFormat::Bgra.has_alpha());
    }

    #[test]
    fn swaps_4byte_channels() {
        let converter = FormatConverter::new(PixelFormat::Bgra, PixelFormat::Rgba);
        assert_eq!(
            converter.format(&[10, 20, 30, 255]).unwrap(),
            vec![30, 20, 10, 255]
        );
        // The reverse direction restores the input.
        let back = FormatConverter::new(PixelFormat::Rgba, PixelFormat::Bgra);
        assert_eq!(
            back.format(&[30, 20, 10, 255]).unwrap(),
            vec![10, 20, 30, 255]
        );
    }

    #[test]
    fn swaps_3byte_channels() {
        let converter = FormatConverter::new(PixelFormat::Rgb, PixelFormat::Bgr);
        assert_eq!(
            converter.format(&[1, 2, 3, 4, 5, 6]).unwrap(),
            vec![3, 2, 1, 6, 5, 4]
        );
    }

    #[test]
    fn expands_with_opaque_alpha() {
        let converter = FormatConverter::new(PixelFormat::Rgb, PixelFormat::Rgba);
        assert_eq!(
            converter.format(&[10, 20, 30, 40, 50, 60]).unwrap(),
            vec![10, 20, 30, 255, 40, 50, 60, 255]
        );
    }

    #[test]
    fn expands_with_channel_swap() {
        let converter = FormatConverter::new(PixelFormat::Rgb, PixelFormat::Bgra);
        assert_eq!(
            converter.format(&[10, 20, 30]).unwrap(),
            vec![30, 20, 10, 255]
        );
        let converter = FormatConverter::new(PixelFormat::Bgr, PixelFormat::Rgba);
        assert_eq!(
            converter.format(&[10, 20, 30]).unwrap(),
            vec![30, 20, 10, 255]
        );
    }

    #[test]
    fn strips_alpha() {
        let converter = FormatConverter::new(PixelFormat::Rgba, PixelFormat::Rgb);
        assert_eq!(
            converter
                .format(&[10, 20, 30, 200, 40, 50, 60, 201])
                .unwrap(),
            vec![10, 20, 30, 40, 50, 60]
        );
    }

    #[test]
    fn strips_alpha_with_channel_swap() {
        // The exact pre-processing chain: BGRA capture -> RGB codec input.
        let to_rgba = FormatConverter::new(PixelFormat::Bgra, PixelFormat::Rgba);
        let rgba = to_rgba.format(&[1, 2, 3, 255, 4, 5, 6, 255]).unwrap();
        assert_eq!(rgba, vec![3, 2, 1, 255, 6, 5, 4, 255]);
        let to_rgb = FormatConverter::new(PixelFormat::Rgba, PixelFormat::Rgb);
        assert_eq!(to_rgb.format(&rgba).unwrap(), vec![3, 2, 1, 6, 5, 4]);

        // Directly: BGRA -> RGB swaps while dropping the alpha.
        let direct = FormatConverter::new(PixelFormat::Bgra, PixelFormat::Rgb);
        assert_eq!(
            direct.format(&[1, 2, 3, 255, 4, 5, 6, 255]).unwrap(),
            vec![3, 2, 1, 6, 5, 4]
        );
    }

    #[test]
    fn preserves_alpha_on_4byte_to_4byte() {
        let converter = FormatConverter::new(PixelFormat::Rgba, PixelFormat::Bgra);
        assert_eq!(converter.format(&[1, 2, 3, 42]).unwrap(), vec![3, 2, 1, 42]);
    }

    #[test]
    fn identity_is_a_copy() {
        for format in [
            PixelFormat::Rgb,
            PixelFormat::Bgr,
            PixelFormat::Rgba,
            PixelFormat::Bgra,
        ] {
            let converter = FormatConverter::new(format, format);
            let src = vec![7u8; format.bytes_per_pixel() * 2];
            assert_eq!(converter.format(&src).unwrap(), src);
        }
    }

    #[test]
    fn all_format_pairs_agree_on_channel_order() {
        // One pixel with distinct channels (R=1, G=2, B=3, A=4): every pair
        // must place R/G/B in the destination order and carry/insert/drop
        // the alpha as documented.
        let src_pixels: [(PixelFormat, Vec<u8>); 4] = [
            (PixelFormat::Rgb, vec![1, 2, 3]),
            (PixelFormat::Bgr, vec![3, 2, 1]),
            (PixelFormat::Rgba, vec![1, 2, 3, 4]),
            (PixelFormat::Bgra, vec![3, 2, 1, 4]),
        ];
        let dst_pixels: [(PixelFormat, Vec<u8>); 4] = [
            (PixelFormat::Rgb, vec![1, 2, 3]),
            (PixelFormat::Bgr, vec![3, 2, 1]),
            (PixelFormat::Rgba, vec![1, 2, 3, 4]),
            (PixelFormat::Bgra, vec![3, 2, 1, 4]),
        ];
        // Opaque expansions insert 255.
        let opaque_dst: [(PixelFormat, Vec<u8>); 2] = [
            (PixelFormat::Rgba, vec![1, 2, 3, 255]),
            (PixelFormat::Bgra, vec![3, 2, 1, 255]),
        ];
        for (src_format, src) in &src_pixels {
            for (dst_format, dst) in &dst_pixels {
                let expected = if !src_format.has_alpha() && dst_format.has_alpha() {
                    &opaque_dst
                        .iter()
                        .find(|(format, _)| format == dst_format)
                        .unwrap()
                        .1
                } else {
                    dst
                };
                let converter = FormatConverter::new(*src_format, *dst_format);
                assert_eq!(
                    converter.format(src).unwrap(),
                    *expected,
                    "{src_format:?} -> {dst_format:?}"
                );
            }
        }
    }

    #[test]
    fn convert_checks_destination_length() {
        let converter = FormatConverter::new(PixelFormat::Rgb, PixelFormat::Rgba);
        let mut dst = vec![0u8; 7];
        let error = converter.convert(&[1, 2, 3], &mut dst).unwrap_err();
        assert_eq!(
            error,
            BufferFormatError::MismatchedDestLength {
                len: 7,
                expected: 4
            }
        );
    }

    #[test]
    fn rejects_partial_source_pixels() {
        let converter = FormatConverter::new(PixelFormat::Rgb, PixelFormat::Rgba);
        let error = converter.format(&[1, 2]).unwrap_err();
        assert_eq!(
            error,
            BufferFormatError::InvalidSourceLength {
                len: 2,
                bytes_per_pixel: 3
            }
        );
    }

    #[test]
    fn in_place_swaps_channels() {
        let converter = FormatConverter::new(PixelFormat::Bgra, PixelFormat::Rgba);
        let mut buf = vec![1, 2, 3, 255, 4, 5, 6, 128];
        converter.convert_in_place(&mut buf).unwrap();
        assert_eq!(buf, vec![3, 2, 1, 255, 6, 5, 4, 128]);
    }

    #[test]
    fn in_place_identity_is_a_no_op() {
        let converter = FormatConverter::new(PixelFormat::Rgb, PixelFormat::Rgb);
        let mut buf = vec![1, 2, 3];
        converter.convert_in_place(&mut buf).unwrap();
        assert_eq!(buf, vec![1, 2, 3]);
    }

    #[test]
    fn in_place_rejects_size_changes() {
        let converter = FormatConverter::new(PixelFormat::Rgb, PixelFormat::Rgba);
        let mut buf = vec![1, 2, 3];
        assert_eq!(
            converter.convert_in_place(&mut buf).unwrap_err(),
            BufferFormatError::InPlaceDifferentPixelSizes {
                source_bytes_per_pixel: 3,
                dest_bytes_per_pixel: 4
            }
        );
    }

    #[test]
    fn fill_black_covers_all_formats() {
        let mut rgb = vec![9u8; 6];
        PixelFormat::Rgb.fill_black(&mut rgb).unwrap();
        assert_eq!(rgb, vec![0; 6]);

        let mut bgr = vec![9u8; 3];
        PixelFormat::Bgr.fill_black(&mut bgr).unwrap();
        assert_eq!(bgr, vec![0; 3]);

        let mut rgba = vec![9u8; 8];
        PixelFormat::Rgba.fill_black(&mut rgba).unwrap();
        assert_eq!(rgba, vec![0, 0, 0, 255, 0, 0, 0, 255]);

        let mut bgra = vec![9u8; 4];
        FormatConverter::new(PixelFormat::Rgb, PixelFormat::Bgra)
            .fill_black(&mut bgra)
            .unwrap();
        assert_eq!(bgra, vec![0, 0, 0, 255]);
    }

    #[test]
    fn error_display_does_not_panic() {
        let error = BufferFormatError::MismatchedDestLength {
            len: 1,
            expected: 4,
        };
        assert!(!format!("{error}").is_empty());
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    enum TestKey {
        Source,
        Dest,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum TestError {
        Failed,
    }

    #[derive(Default)]
    struct TestFrame {
        buffers: HashMap<TestKey, BytesMut>,
        error: Option<TestError>,
    }

    impl BorrowMutFrameProperties<TestKey, BytesMut> for TestFrame {
        fn get_mut_ref(&mut self, key: &TestKey) -> Option<&mut BytesMut> {
            self.buffers.get_mut(key)
        }
    }

    impl FrameError<TestError> for TestFrame {
        fn report_error(&mut self, error: TestError) {
            self.error = Some(error);
        }

        fn get_error(&self) -> Option<TestError> {
            self.error
        }
    }

    fn frame_with(source: &[u8]) -> TestFrame {
        let mut frame = TestFrame::default();
        frame
            .buffers
            .insert(TestKey::Source, BytesMut::from(source));
        frame
            .buffers
            .insert(TestKey::Dest, BytesMut::with_capacity(64));
        frame
    }

    #[tokio::test]
    async fn stage_converts_between_buffers() {
        let mut stage = BufferFormatter::source(TestKey::Source, PixelFormat::Rgb)
            .dest(TestKey::Dest, PixelFormat::Rgba);
        let mut frame = stage
            .process(frame_with(&[10, 20, 30, 40, 50, 60]))
            .await
            .unwrap();

        let dest = frame.get_mut_ref(&TestKey::Dest).unwrap();
        assert_eq!(dest.as_ref(), &[10, 20, 30, 255, 40, 50, 60, 255]);
        // The source buffer is preserved for upstream redeemers.
        assert_eq!(
            frame.get_mut_ref(&TestKey::Source).unwrap().as_ref(),
            &[10, 20, 30, 40, 50, 60]
        );
    }

    #[tokio::test]
    async fn stage_converts_in_place_on_a_shared_key() {
        let mut stage = BufferFormatter::source(TestKey::Source, PixelFormat::Bgra)
            .dest(TestKey::Source, PixelFormat::Rgba);
        let mut frame = stage.process(frame_with(&[1, 2, 3, 255])).await.unwrap();

        assert_eq!(
            frame.get_mut_ref(&TestKey::Source).unwrap().as_ref(),
            &[3, 2, 1, 255]
        );
    }

    #[tokio::test]
    async fn stage_skips_errored_frames_leaving_the_destination_untouched() {
        let mut stage = BufferFormatter::source(TestKey::Source, PixelFormat::Rgb)
            .dest(TestKey::Dest, PixelFormat::Rgba);
        let mut frame = frame_with(&[10, 20, 30]);
        frame.report_error(TestError::Failed);

        let mut frame = stage.process(frame).await.unwrap();
        assert_eq!(frame.get_mut_ref(&TestKey::Dest).unwrap().len(), 0);
    }

    #[tokio::test]
    async fn stage_skips_a_missing_source_leaving_the_destination_untouched() {
        let mut stage = BufferFormatter::source(TestKey::Source, PixelFormat::Rgb)
            .dest(TestKey::Dest, PixelFormat::Rgba);
        let mut frame = TestFrame::default();
        frame
            .buffers
            .insert(TestKey::Dest, BytesMut::with_capacity(64));

        let mut frame = stage.process(frame).await.unwrap();
        assert_eq!(frame.get_mut_ref(&TestKey::Dest).unwrap().len(), 0);
    }

    #[tokio::test]
    async fn stage_skips_a_partial_source_leaving_the_destination_untouched() {
        let mut stage = BufferFormatter::source(TestKey::Source, PixelFormat::Rgb)
            .dest(TestKey::Dest, PixelFormat::Rgba);
        let mut frame = stage.process(frame_with(&[10, 20])).await.unwrap();
        assert_eq!(frame.get_mut_ref(&TestKey::Dest).unwrap().len(), 0);
    }

    #[tokio::test]
    async fn fill_stage_paints_errored_frames_opaque_black() {
        let mut stage = FillBlack::new(TestKey::Dest, PixelFormat::Rgba, 8);
        let mut frame = TestFrame::default();
        frame
            .buffers
            .insert(TestKey::Dest, BytesMut::from(&[9u8; 8][..]));
        frame.report_error(TestError::Failed);

        let mut frame = stage.process(frame).await.unwrap();
        assert_eq!(
            frame.get_mut_ref(&TestKey::Dest).unwrap().as_ref(),
            &[0, 0, 0, 255, 0, 0, 0, 255]
        );
    }

    #[tokio::test]
    async fn fill_stage_zero_fills_an_empty_destination_on_healthy_frames() {
        let mut stage = FillBlack::new(TestKey::Dest, PixelFormat::Rgba, 8);
        let mut frame = TestFrame::default();
        frame.buffers.insert(TestKey::Dest, BytesMut::new());

        let mut frame = stage.process(frame).await.unwrap();
        assert_eq!(frame.get_mut_ref(&TestKey::Dest).unwrap().as_ref(), &[0; 8]);
    }

    #[tokio::test]
    async fn fill_stage_leaves_converted_pixels_untouched() {
        let mut stage = FillBlack::new(TestKey::Dest, PixelFormat::Rgba, 8);
        let mut frame = TestFrame::default();
        frame
            .buffers
            .insert(TestKey::Dest, BytesMut::from(&[10u8, 20, 30, 255][..]));

        let mut frame = stage.process(frame).await.unwrap();
        assert_eq!(
            frame.get_mut_ref(&TestKey::Dest).unwrap().as_ref(),
            &[10, 20, 30, 255]
        );
    }
}
