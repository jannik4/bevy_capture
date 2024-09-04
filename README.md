# bevy_capture

[![crates.io](https://img.shields.io/crates/v/bevy_capture.svg)](https://crates.io/crates/bevy_capture)
[![docs.rs](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/bevy_capture)

A Bevy plugin for capturing frames from a Bevy application. It comes with some built-in encoders, e.g. for creating gifs or videos, and can be easily extended with custom encoders.

## Current Limitations

- Only headless rendering is supported, but windowed rendering should be possible as well. PRs are welcome!
- There isn't a built-in method to determine when everything is ready (such as assets loaded and pipelines built).
  The best approach is to wait a few frames before starting the capture.

## Built-in Encoders

| Name                                                                  | Description                                                               | Required Features |
| --------------------------------------------------------------------- | ------------------------------------------------------------------------- | ----------------- |
| [`FramesEncoder`](encoder::frames::FramesEncoder)                     | Encodes frames into individual images.                                    |                   |
| [`GifEncoder`](encoder::gif::GifEncoder)                              | Encodes frames into a gif.                                                | `gif`             |
| [`Mp4Openh264Encoder`](encoder::mp4_openh264::Mp4Openh264Encoder)     | Encodes frames into an mp4 using openh264.                                | `mp4_openh264`    |
| [`Mp4FfmpegCliEncoder`](encoder::mp4_ffmpeg_cli::Mp4FfmpegCliEncoder) | Encodes frames into an mp4 using the ffmpeg CLI (ffmpeg must be in PATH). | `mp4_ffmpeg_cli`  |

## Usage

For a complete example, see the [simple example](https://github.com/jannik4/bevy_capture/blob/main/examples/simple.rs).

```rust,ignore
// Add plugins
app.add_plugins((
    // Disable the WinitPlugin to prevent the creation of a window
    DefaultPlugins.build().disable::<WinitPlugin>(),
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
      Camera2dBundle::default().target_headless(512, 512, &mut images),
      CaptureBundle::default(),
  ));
}

// Start capturing
fn update(mut capture: Query<&mut Capture>) {
  let mut capture = capture.single_mut();
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
