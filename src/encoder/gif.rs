//! Encodes frames into a gif.

use super::{Encoder, Result};
use bevy::prelude::*;
use image::{codecs::gif, Frame};
use std::io::Write;

pub use gif::Repeat;

/// An encoder that encodes a sequence of images into a gif.
pub struct GifEncoder<W: Write>(gif::GifEncoder<W>);

impl<W: Write> GifEncoder<W> {
    /// Creates a new gif encoder that writes the gif to the given writer, e.g. a file.
    pub fn new(writer: W) -> Self {
        Self(gif::GifEncoder::new(writer))
    }

    /// Creates a new gif encoder that writes the gif to the given writer, e.g. a file,
    /// with the given speed.
    /// See [`Frame::from_rgba_speed`](https://docs.rs/gif/latest/gif/struct.Frame.html#method.from_rgba_speed)
    /// for more information on the speed parameter.
    pub fn new_with_speed(writer: W, speed: i32) -> Self {
        Self(gif::GifEncoder::new_with_speed(writer, speed))
    }

    /// Sets the repeat mode of the gif.
    pub fn with_repeat(mut self, repeat: Repeat) -> Self {
        self.0.set_repeat(repeat).unwrap();
        self
    }
}

impl<W: Write> Encoder for GifEncoder<W> {
    fn encode(&mut self, image: &Image) -> Result<()> {
        let image = image.clone().try_into_dynamic()?;
        let buffer = image.to_rgba8();
        self.0.encode_frame(Frame::new(buffer))?;
        Ok(())
    }
}
