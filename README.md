# bevy_capture

[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](#license)
[![Build Status](https://github.com/jannik4/bevy_capture/workflows/CI/badge.svg)](https://github.com/jannik4/bevy_capture/actions)
[![crates.io](https://img.shields.io/crates/v/bevy_capture.svg)](https://crates.io/crates/bevy_capture)
[![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/bevy_capture)

A Bevy plugin for capturing frames from a Bevy application. It comes with some built-in encoders, e.g. for creating gifs or videos, and can be easily extended with custom encoders.

## Current Limitations

- Only headless rendering is supported, but windowed rendering should be possible as well. PRs are welcome!

## Built-in Encoders

| Name                                                                  | Description                                                               | Required Features |
| --------------------------------------------------------------------- | ------------------------------------------------------------------------- | ----------------- |
| [`FramesEncoder`](encoder::frames::FramesEncoder)                     | Encodes frames into individual images.                                    |                   |
| [`GifEncoder`](encoder::gif::GifEncoder)                              | Encodes frames into a gif.                                                | `gif`             |
| [`Mp4Openh264Encoder`](encoder::mp4_openh264::Mp4Openh264Encoder)     | Encodes frames into an mp4 using openh264.                                | `mp4_openh264`    |
| [`Mp4FfmpegCliEncoder`](encoder::mp4_ffmpeg_cli::Mp4FfmpegCliEncoder) | Encodes frames into an mp4 using the ffmpeg CLI (ffmpeg must be in PATH). | `mp4_ffmpeg_cli`  |
| [`Mp4FfmpegCliPipeEncoder`](encoder::streaming_ffmpeg_cli_pipe::Mp4FfmpegCliPipeEncoder) | Encodes frames into an mp4 by streaming it directly to the ffmpeg CLI (ffmpeg must be in PATH). | `mp4_ffmpeg_cli_pipe`  |

## Usage

For a complete example, see the [simple example](https://github.com/jannik4/bevy_capture/blob/main/examples/simple.rs).

```rust,ignore
// Add plugins
app.add_plugins((
    DefaultPlugins
        .build()
        // Disable the WinitPlugin to prevent the creation of a window
        .disable::<WinitPlugin>()
        // Make sure pipelines are ready before rendering
        .set(RenderPlugin {
            synchronous_pipeline_compilation: true,
            ..default()
        }),
    // Add the ScheduleRunnerPlugin to run the app in loop mode
    ScheduleRunnerPlugin {
        run_mode: RunMode::Loop { wait: None },
    },
    // Add the CapturePlugin
    bevy_capture::CapturePlugin,
));

// Spawn a camera with the CaptureBundle
fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
  commands.spawn((
      Camera2d,
      Camera::default().target_headless(512, 512, &mut images),
      CaptureBundle::default(),
  ));
}

// Start capturing
fn update(mut capture: Query<&mut Capture>, mut waited: Local<bool>) {
  // Wait one frame: https://github.com/bevyengine/bevy/issues/20756
  if !*waited {
      *waited = true;
      return;
  }

  let mut capture = capture.single_mut().unwrap();
  if !capture.is_capturing() {
    capture.start(
      GifEncoder::new(File::create("my_capture.gif").unwrap())
        .with_repeat(gif::Repeat::Infinite)
    );
  }
}
```

## Implementing a Custom Encoder

```rust,ignore
struct MyCustomEncoder;

impl Encoder for MyCustomEncoder {
    fn encode(&mut self, image: &Image) -> Result<()> {
        // Called for each frame.
        todo!("Encode the image into your custom format.")
    }

    fn finish(self: Box<Self>) {
      // Called when the encoder is stopped.
      todo!("Finish encoding the frames, if necessary.")
    }
}
```

## Alternatives

- [bevy_image_export](https://github.com/paulkre/bevy_image_export): Less opinionated, no encoders included, only image sequences. This might be a better fit, if you end up using ffmpeg on the frames anyway.

## License

Licensed under either of

- Apache License, Version 2.0
  ([LICENSE-APACHE-2.0](LICENSE-Apache-2.0) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT License
  ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
