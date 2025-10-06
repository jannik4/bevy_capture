//! MP4 encoder using ffmpeg CLI with real-time streaming (ffmpeg must be in PATH).

use super::{Encoder, Result};
use bevy::prelude::*;
use std::{
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::Duration,
};

/// An encoder that streams frames directly to ffmpeg for real-time MP4 encoding.
/// ffmpeg must be in PATH.
pub struct Mp4FfmpegCliPipeEncoder {
    /// The ffmpeg child process
    process: Option<Child>,

    /// Output file path
    path: PathBuf,

    /// Video configuration
    framerate: u32,
    crf: u32,
    preset: String,

    /// Hardware encoder preference (nvenc, vaapi, etc.)
    hardware_encoder: Option<String>,

    /// Video resolution (width, height)
    resolution: Option<(u32, u32)>,

    /// Flag to track if finish() was called
    finished: bool,
}

impl Mp4FfmpegCliPipeEncoder {
    /// Creates a new MP4 encoder that writes the MP4 to the given path.
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        Ok(Self {
            process: None,
            path: path.into(),
            framerate: 60,
            crf: 23,
            preset: "fast".to_string(),
            hardware_encoder: None,
            resolution: None,
            finished: false,
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

    /// Sets the preset of the video.
    pub fn with_preset(mut self, preset: impl Into<String>) -> Self {
        self.preset = preset.into();
        self
    }

    /// Sets hardware encoder (e.g., "h264_nvenc", "h264_vaapi", "h264_videotoolbox").
    /// If the hardware encoder is not available, it will fallback to libx264.
    pub fn with_hardware_encoder(mut self, encoder: impl Into<String>) -> Self {
        self.hardware_encoder = Some(encoder.into());
        self
    }

    /// Explicitly sets the video resolution. If not set, resolution will be determined from the first frame.
    pub fn with_resolution(mut self, width: u32, height: u32) -> Self {
        self.resolution = Some((width, height));
        self
    }

    /// Detects available hardware encoders on the system
    pub fn detect_hardware_encoder() -> Option<String> {
        // Try NVIDIA NVENC first
        if Self::test_encoder("h264_nvenc") {
            return Some("h264_nvenc".to_string());
        }

        // Try Intel Quick Sync Video
        if Self::test_encoder("h264_qsv") {
            return Some("h264_qsv".to_string());
        }

        // Try VAAPI (Linux)
        if Self::test_encoder("h264_vaapi") {
            return Some("h264_vaapi".to_string());
        }

        // Try VideoToolbox (macOS)
        if Self::test_encoder("h264_videotoolbox") {
            return Some("h264_videotoolbox".to_string());
        }

        None
    }

    /// Tests if a specific encoder is available
    fn test_encoder(encoder: &str) -> bool {
        Command::new("ffmpeg")
            .args(["-hide_banner", "-encoders"])
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).contains(encoder))
            .unwrap_or(false)
    }

    /// Initializes the ffmpeg process with the first frame to determine video properties
    fn initialize_process(&mut self, image: &Image) -> Result<()> {
        let (width, height) = match self.resolution {
            Some((w, h)) => (w, h),
            None => (image.width(), image.height()),
        };

        // Preset
        let preset = &self.preset;

        // Choose encoder
        let encoder = self
            .hardware_encoder
            .clone()
            .or_else(Self::detect_hardware_encoder)
            .unwrap_or_else(|| "libx264".to_string());

        bevy::log::info!("Using encoder: {} for {}x{} video", encoder, width, height);

        let mut command = Command::new("ffmpeg");

        // Input settings
        command.args([
            "-y", // Overwrite output file
            "-f",
            "rawvideo", // Input format
            "-pix_fmt",
            "rgba", // Input pixel format (Bevy uses RGBA)
            "-s",
            &format!("{}x{}", width, height), // Input resolution
            "-r",
            &self.framerate.to_string(), // Input framerate
            "-i",
            "pipe:0", // Read from stdin
        ]);

        // Encoder settings
        command.args(["-c:v", &encoder]);

        // Pixel format for output
        command.args(["-pix_fmt", "yuv420p"]);

        // Quality settings - adjust based on encoder type
        match encoder.as_str() {
            "h264_nvenc" => {
                command.args(["-preset", preset]);
                command.args(["-cq", &self.crf.to_string()]);
            }
            "h264_qsv" => {
                command.args(["-preset", preset]);
                command.args(["-global_quality", &self.crf.to_string()]);
            }
            "h264_vaapi" => {
                command.args(["-vaapi_device", "/dev/dri/renderD128"]);
                command.args(["-qp", &self.crf.to_string()]);
            }
            "h264_videotoolbox" => {
                command.args(["-q:v", &self.crf.to_string()]);
            }
            _ => {
                // Default libx264 settings
                command.args(["-preset", preset]);
                command.args(["-crf", &self.crf.to_string()]);
            }
        }

        // Output file
        command.arg(&self.path);

        // Set up pipes
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::null()) // Ignore stdout
            .stderr(Stdio::piped()); // Capture stderr for error messages

        let mut child = command.spawn()?;
        // Drain stderr on a background thread to avoid pipe backpressure.
        // ffmpeg writes logs to stderr; if the parent never reads them the kernel pipe buffer can fill
        // and ffmpeg will block on writes, preventing it from exiting.
        if let Some(stderr) = child.stderr.take() {
            std::thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    match line {
                        Ok(l) => bevy::log::debug!("ffmpeg: {}", l),
                        Err(e) => {
                            bevy::log::warn!("failed reading ffmpeg stderr: {}", e);
                            break;
                        }
                    }
                }
            });
        }
        self.process = Some(child);

        Ok(())
    }

    /// Converts Bevy Image (RGBA) to raw bytes suitable for ffmpeg
    fn image_to_raw_bytes(image: &Image) -> Result<&Vec<u8>> {
        // Bevy images are typically in RGBA format
        let image_data = image.data.as_ref().ok_or("Image has no data")?;

        // ffmpeg expects raw RGBA bytes
        Ok(image_data)
    }

    /// Logs ffmpeg error details from stderr
    fn log_ffmpeg_error(&self, process: &mut Child, status: std::process::ExitStatus) {
        if let Some(mut stderr) = process.stderr.take() {
            use std::io::Read;
            let mut error_msg = String::new();
            if stderr.read_to_string(&mut error_msg).is_ok() {
                bevy::log::error!("FFmpeg failed with status {}: {}", status, error_msg);
            } else {
                bevy::log::error!("FFmpeg failed with status: {}", status);
            }
        } else {
            bevy::log::error!("FFmpeg failed with status: {}", status);
        }
    }

    /// Internal cleanup method shared by finish() and Drop
    fn cleanup(&mut self, graceful: bool) {
        if let Some(mut process) = self.process.take() {
            // Close stdin to signal end of input
            drop(process.stdin.take());

            if graceful {
                // Graceful shutdown with timeout
                match wait_timeout::ChildExt::wait_timeout(&mut process, Duration::from_secs(10)) {
                    Ok(Some(status)) => {
                        // Process finished within timeout
                        if status.success() {
                            bevy::log::info!(
                                "Video encoding completed successfully: {:?}",
                                self.path
                            );
                        } else {
                            self.log_ffmpeg_error(&mut process, status);
                        }
                    }
                    Ok(None) => {
                        // Timeout exceeded - process still running
                        bevy::log::warn!("FFmpeg encoding timeout after 10s, forcing termination");
                        let _ = process.kill();
                        let _ = process.wait();
                    }
                    Err(e) => {
                        // System error while waiting
                        bevy::log::error!("Failed to wait for FFmpeg process: {:?}", e);
                        let _ = process.kill(); // Force kill for safety
                    }
                }
            } else {
                // Forceful shutdown (from Drop)
                let _ = process.kill();
                let _ = process.wait();
            }
        }
    }
}

impl Encoder for Mp4FfmpegCliPipeEncoder {
    fn encode(&mut self, image: &Image) -> Result<()> {
        // Initialize process on first frame
        if self.process.is_none() {
            self.initialize_process(image)?;
        }

        // Get the stdin pipe
        let process = self
            .process
            .as_mut()
            .ok_or("FFmpeg process not initialized")?;

        let stdin = process.stdin.as_mut().ok_or("Failed to get stdin pipe")?;

        // Convert image to raw bytes and write to ffmpeg
        let raw_bytes = Self::image_to_raw_bytes(image)?;
        stdin.write_all(raw_bytes)?;
        stdin.flush()?;

        Ok(())
    }

    fn finish(mut self: Box<Self>) {
        self.finished = true;
        self.cleanup(true);
    }
}

impl Drop for Mp4FfmpegCliPipeEncoder {
    fn drop(&mut self) {
        // Only cleanup if finish() wasn't called
        if !self.finished && self.process.is_some() {
            bevy::log::warn!(
                "Mp4FfmpegCliPipeEncoder dropped without calling finish(), forcing cleanup"
            );
            self.cleanup(false);
        }
    }
}
