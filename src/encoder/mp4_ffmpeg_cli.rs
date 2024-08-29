//! MP4 encoder using ffmpeg CLI (ffmpeg must be in PATH).

use super::{Encoder, Result};
use bevy::prelude::*;
use std::{path::PathBuf, process::Command};
use tempdir::TempDir;

/// An encoder that encodes a sequence of images into an MP4 file using ffmpeg CLI.
/// ffmpeg must be in PATH.
pub struct Mp4FfmpegCliEncoder {
    dir: TempDir,
    frame: u32,
    path: PathBuf,

    framerate: u32,
    crf: u32,
}

impl Mp4FfmpegCliEncoder {
    /// Creates a new MP4 encoder that writes the MP4 to the given path.
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        Ok(Self {
            dir: TempDir::new("bevy_capture")?,
            frame: 0,
            path: path.into(),

            framerate: 60,
            crf: 23,
        })
    }

    /// Sets the framerate of the video.
    pub fn with_framerate(mut self, framerate: u32) -> Self {
        self.framerate = framerate;
        self
    }

    /// Sets the CRF (Constant Rate Factor) of the video.
    pub fn with_crf(mut self, crf: u32) -> Self {
        self.crf = crf;
        self
    }
}

impl Encoder for Mp4FfmpegCliEncoder {
    fn encode(&mut self, image: &Image) -> Result<()> {
        let image = image.clone().try_into_dynamic()?;
        image.save(self.dir.path().join(format!("frame_{:06}.png", self.frame)))?;

        self.frame += 1;

        Ok(())
    }

    fn finish(self: Box<Self>) {
        let mut command;
        if cfg!(target_os = "windows") {
            command = Command::new("cmd");
            command.arg("/C");
        } else {
            command = Command::new("sh");
            command.arg("-c");
        };

        command.arg("ffmpeg");
        command.arg("-framerate").arg(self.framerate.to_string());
        command
            .arg("-i")
            .arg(self.dir.path().join("frame_%06d.png"));
        command.arg("-c:v").arg("libx264");
        command.arg("-pix_fmt").arg("yuv420p");
        command.arg("-crf").arg(self.crf.to_string());
        command.arg(self.path);

        match command.output() {
            Ok(output) => {
                if !output.status.success() {
                    bevy::log::error!("ffmpeg failed: {:?}", output);
                }
            }
            Err(error) => {
                bevy::log::error!("ffmpeg failed: {:?}", error);
            }
        }
    }
}
