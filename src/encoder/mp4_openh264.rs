//! MP4 encoder using OpenH264.

use super::{Encoder, Result};
use bevy::prelude::*;
use image::RgbaImage;
use mp4::{
    AvcConfig, FourCC, MediaConfig, Mp4Config, Mp4Sample, Mp4Writer, TrackConfig, TrackType,
};
use openh264::{
    encoder::{EncoderConfig, FrameType},
    formats::{RGBSource, YUVBuffer},
    OpenH264API, Timestamp,
};
use std::{
    io::{Seek, Write},
    str::FromStr,
};

type Openh264Encoder = openh264::encoder::Encoder;

pub use openh264;

/// An encoder that encodes a sequence of images into an MP4 file using OpenH264.
pub struct Mp4Openh264Encoder<W> {
    mp4: Mp4Writer<W>,
    mp4_track_added: bool,
    openh264: Openh264Encoder,
    frame: u64,
    width: u16,
    height: u16,
}

impl<W: Write + Seek> Mp4Openh264Encoder<W> {
    /// Creates a new MP4 encoder that writes the MP4 to the given writer, e.g. a file.
    /// The width and height of the video should match the dimensions of the images.
    pub fn new(writer: W, width: u16, height: u16) -> Result<Self> {
        Self::new_with_config(writer, width, height, EncoderConfig::new())
    }

    /// Creates a new MP4 encoder that writes the MP4 to the given writer, e.g. a file.
    /// The width and height of the video should match the dimensions of the images.
    /// The encoder configuration can be used to set the desired quality and other parameters.
    pub fn new_with_config(
        writer: W,
        width: u16,
        height: u16,
        config: EncoderConfig,
    ) -> Result<Self> {
        let mp4 = Mp4Writer::write_start(
            writer,
            &Mp4Config {
                major_brand: FourCC::from_str("isom").unwrap(),
                minor_version: 512,
                compatible_brands: vec![
                    FourCC::from_str("isom").unwrap(),
                    FourCC::from_str("iso2").unwrap(),
                    FourCC::from_str("avc1").unwrap(),
                    FourCC::from_str("mp41").unwrap(),
                ],
                timescale: 1000,
            },
        )?;

        Ok(Self {
            mp4,
            mp4_track_added: false,
            openh264: Openh264Encoder::with_api_config(OpenH264API::from_source(), config)?,
            frame: 0,
            width,
            height,
        })
    }
}

impl<W: Write + Seek> Encoder for Mp4Openh264Encoder<W> {
    fn encode(&mut self, image: &Image) -> Result<()> {
        let image = image.clone().try_into_dynamic()?;
        let buffer = image.to_rgba8();

        let bitstream = self.openh264.encode_at(
            &YUVBuffer::from_rgb_source(ImageSource(buffer)),
            Timestamp::from_millis(self.frame * 100),
        )?;

        if !self.mp4_track_added {
            let layer_0 = bitstream.layer(0).unwrap();
            self.mp4.add_track(&TrackConfig {
                track_type: TrackType::Video,
                timescale: 1000,
                language: "und".to_string(),
                media_conf: MediaConfig::AvcConfig(AvcConfig {
                    width: self.width,
                    height: self.height,
                    seq_param_set: remove_nal_start_code(layer_0.nal_unit(0).unwrap()).to_vec(),
                    pic_param_set: remove_nal_start_code(layer_0.nal_unit(1).unwrap()).to_vec(),
                }),
            })?;
            self.mp4_track_added = true;
        }

        let mut bytes = Vec::new();
        for l in 0..bitstream.num_layers() {
            let layer = bitstream.layer(l).unwrap();
            if layer.is_video() {
                for n in 0..layer.nal_count() {
                    let nal = remove_nal_start_code(layer.nal_unit(n).unwrap());
                    bytes.extend_from_slice(&u32::to_be_bytes(nal.len() as u32));
                    bytes.extend_from_slice(nal);
                }
            }
        }

        self.mp4.write_sample(
            1,
            &Mp4Sample {
                start_time: self.frame * 100,
                duration: 100,
                rendering_offset: 0,
                is_sync: matches!(bitstream.frame_type(), FrameType::I | FrameType::IDR),
                bytes: bytes.into(),
            },
        )?;

        self.frame += 1;
        Ok(())
    }

    fn finish(mut self: Box<Self>) {
        if let Err(err) = self.mp4.write_end() {
            bevy::log::error!("Failed to write mp4 end: {}", err);
        }
    }
}

struct ImageSource(RgbaImage);

impl RGBSource for ImageSource {
    fn dimensions(&self) -> (usize, usize) {
        (self.0.width() as usize, self.0.height() as usize)
    }

    fn pixel_f32(&self, x: usize, y: usize) -> (f32, f32, f32) {
        let [r, g, b, _] = self.0.get_pixel(x as u32, y as u32).0;
        (r as f32, g as f32, b as f32)
    }
}

fn remove_nal_start_code(nal: &[u8]) -> &[u8] {
    if nal.starts_with(&[0, 0, 0, 1]) {
        &nal[4..]
    } else if nal.starts_with(&[0, 0, 1]) {
        &nal[3..]
    } else {
        nal
    }
}
