//! Encoders for different formats.

pub mod frames;

#[cfg(feature = "gif")]
pub mod gif;

#[cfg(feature = "mp4_openh264")]
pub mod mp4_openh264;

#[cfg(feature = "mp4_ffmpeg_cli")]
pub mod mp4_ffmpeg_cli;

use bevy::prelude::*;

/// An error that occurred during encoding.
pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

/// The result type for encoding operations.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An encoder that encodes a sequence of images into a custom format.
pub trait Encoder {
    /// Encodes the given image.
    fn encode(&mut self, image: &Image) -> Result<()>;

    /// Finishes the encoding process.
    /// This method can be used to finalize the encoding process and write any remaining data, if necessary.
    fn finish(self: Box<Self>) {}
}
