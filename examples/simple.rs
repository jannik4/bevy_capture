use bevy::{
    app::{RunMode, ScheduleRunnerPlugin},
    prelude::*,
    render::RenderPlugin,
    time::TimeUpdateStrategy,
    winit::WinitPlugin,
};
use bevy_capture::{
    encoder::{frames, gif, mp4_ffmpeg_cli, mp4_ffmpeg_cli_pipe, mp4_openh264},
    CameraTargetHeadless, Capture, CaptureBundle,
};
use std::{f32::consts::TAU, fs, time::Duration};

fn main() -> AppExit {
    // Create the captures directory
    fs::create_dir_all("captures/simple").unwrap();

    let mut app = App::new();

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

    // Update the time at a fixed rate of 60 FPS
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / 60.0,
    )));

    // Setup
    app.add_systems(Startup, setup);

    // Update
    app.add_systems(Update, update);

    // Run the app
    app.run()
}

#[derive(Component)]
struct Cube;

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn((
        Camera2d,
        Camera::default().target_headless(512, 512, &mut images),
        CaptureBundle::default(),
    ));

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(128.0, 128.0))),
        MeshMaterial2d(materials.add(Color::srgb(0.0, 0.0, 1.0))),
        Cube,
    ));
}

fn update(
    mut app_exit: MessageWriter<AppExit>,
    mut capture: Query<&mut Capture>,
    mut cubes: Query<&mut Transform, With<Cube>>,
    mut frame: Local<u32>,
    mut waited: Local<bool>,
) {
    // Wait one frame: https://github.com/bevyengine/bevy/issues/20756
    if !*waited {
        *waited = true;
        return;
    }

    let mut capture = capture.single_mut().unwrap();
    if !capture.is_capturing() {
        capture.start((
            gif::GifEncoder::new(fs::File::create("captures/simple/simple.gif").unwrap())
                .with_repeat(gif::Repeat::Infinite),
            frames::FramesEncoder::new("captures/simple/frames"),
            mp4_ffmpeg_cli::Mp4FfmpegCliEncoder::new("captures/simple/simple_ffmpeg.mp4")
                .unwrap()
                .with_framerate(10),
            mp4_ffmpeg_cli_pipe::Mp4FfmpegCliPipeEncoder::new(
                "captures/simple/simple_ffmpeg_pipe.mp4",
            )
            .expect("Failed to create MP4 encoder")
            .with_framerate(10)
            .with_crf(18)
            .with_preset("p7".to_string()),
            mp4_openh264::Mp4Openh264Encoder::new(
                fs::File::create("captures/simple/simple_openh264.mp4").unwrap(),
                512,
                512,
            )
            .unwrap(),
        ));
    }

    for mut transform in &mut cubes {
        transform.rotation = Quat::from_rotation_z(*frame as f32 / 60.0 * TAU)
    }

    *frame += 1;
    if *frame >= 15 {
        app_exit.write(AppExit::Success);
    }
}
