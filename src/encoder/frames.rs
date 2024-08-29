//! Encode frames into individual images;

use super::{Encoder, Result};
use bevy::prelude::*;
use std::{fs, path::PathBuf};

/// An encoder that encodes a sequence of images into individual images.
pub struct FramesEncoder {
    path: PathBuf,
    frame: u32,
}

impl FramesEncoder {
    /// Creates a new frames encoder that writes frames to the given directory.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            frame: 0,
        }
    }
}

impl Encoder for FramesEncoder {
    fn encode(&mut self, image: &Image) -> Result<()> {
        fs::create_dir_all(&self.path)?;

        let image = image.clone().try_into_dynamic()?;
        image.save(self.path.join(format!("frame_{:06}.png", self.frame)))?;

        self.frame += 1;

        Ok(())
    }
}
