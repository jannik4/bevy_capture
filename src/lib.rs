#![deny(rust_2018_idioms)]
#![allow(elided_lifetimes_in_paths)]
#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

mod render_world;

pub mod encoder;

use bevy::{
    asset::RenderAssetUsages,
    camera::RenderTarget,
    image::BevyDefault,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
};
use std::sync::Mutex;
use variadics_please::all_tuples;

#[doc(inline)]
pub use encoder::Encoder;

type BoxedEncoder = Box<dyn Encoder + Send + Sync + 'static>;

/// A Bevy plugin for capturing frames.
pub struct CapturePlugin;

impl Plugin for CapturePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(render_world::CaptureRenderWorldPlugin);
    }
}

/// Bundle for the capture plugin. This is usually attached to a camera.
#[derive(Default, Bundle)]
pub struct CaptureBundle {
    /// The capture component.
    pub capture: Capture,
    /// The source of the capture.
    pub camera_source: CaptureSource,
}

/// The capture component.
#[derive(Default, Component)]
pub struct Capture {
    state: CaptureState,
}

impl Capture {
    /// Starts capturing frames with the given encoders.
    pub fn start(&mut self, encoders: impl IntoEncoders) {
        self.state = CaptureState::Capturing {
            encoders: Mutex::new(Some(Encoders(encoders.into_encoders()))),
            paused: false,
        };
    }

    /// Pauses the capture.
    pub fn pause(&mut self) {
        if let CaptureState::Capturing { paused, .. } = &mut self.state {
            *paused = true;
        }
    }

    /// Resumes the capture.
    pub fn resume(&mut self) {
        if let CaptureState::Capturing { paused, .. } = &mut self.state {
            *paused = false;
        }
    }

    /// Stops the capture. This will drop the active encoders, which will call [`finish`](Encoder::finish)
    /// on them.
    pub fn stop(&mut self) {
        self.state = CaptureState::Idle;
    }

    /// Returns `true` if the capture is currently capturing frames.
    pub fn is_capturing(&self) -> bool {
        matches!(&self.state, CaptureState::Capturing { .. })
    }

    /// Returns `true` if the capture is currently paused.
    pub fn is_paused(&self) -> bool {
        matches!(&self.state, CaptureState::Capturing { paused: true, .. })
    }
}

#[derive(Default)]
enum CaptureState {
    #[default]
    Idle,
    Capturing {
        encoders: Mutex<Option<Encoders>>,
        paused: bool,
    },
}

struct Encoders(Vec<BoxedEncoder>);

impl Drop for Encoders {
    fn drop(&mut self) {
        for encoder in self.0.drain(..) {
            encoder.finish();
        }
    }
}

/// The source of the capture.
#[derive(Default, Clone, Copy, Component)]
#[non_exhaustive] // TODO: For windowed rendering: MainWindow, Window(Entity)
pub enum CaptureSource {
    /// Use the camera of the entity this component is attached to.
    #[default]
    ThisCamera,
    /// Use the camera with the given entity.
    Camera(Entity),
}

/// Extension trait for the camera to set the target to a headless image.
///
/// # Example
/// ```ignore
/// # use bevy::prelude::*;
/// # use bevy_capture::CameraTargetHeadless;
/// #
/// fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>) {
///    commands.spawn((Camera2d, Camera::default().target_headless(512, 512, &mut images)));
/// }
/// ```
pub trait CameraTargetHeadless {
    /// Sets the target of the camera to a headless image with the given dimensions.
    fn target_headless(self, width: u32, height: u32, images: &mut Assets<Image>) -> Self;
}

impl CameraTargetHeadless for Camera {
    fn target_headless(mut self, width: u32, height: u32, images: &mut Assets<Image>) -> Self {
        let mut image = Image::new_fill(
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0; 4],
            TextureFormat::bevy_default(),
            RenderAssetUsages::default(),
        );
        image.texture_descriptor.usage |= TextureUsages::COPY_SRC
            | TextureUsages::RENDER_ATTACHMENT
            | TextureUsages::TEXTURE_BINDING;

        self.target = RenderTarget::Image(images.add(image).into());

        self
    }
}

/// Convert a value into a sequence of encoders.
pub trait IntoEncoders {
    /// Converts the value into a sequence of encoders.
    fn into_encoders(self) -> Vec<BoxedEncoder>;
}

impl IntoEncoders for BoxedEncoder {
    fn into_encoders(self) -> Vec<BoxedEncoder> {
        vec![self]
    }
}

impl IntoEncoders for Vec<BoxedEncoder> {
    fn into_encoders(self) -> Vec<BoxedEncoder> {
        self
    }
}

impl<E> IntoEncoders for E
where
    E: Encoder + Send + Sync + 'static,
{
    fn into_encoders(self) -> Vec<BoxedEncoder> {
        vec![Box::new(self)]
    }
}

macro_rules! impl_into_encoders {
    ($(($E:ident, $e:ident)),*) => {
        impl<$($E),*> IntoEncoders for ($($E,)*)
        where
            $($E: Encoder + Send + Sync + 'static,)*
        {
            fn into_encoders(self) -> Vec<BoxedEncoder> {
                let ($($e,)*) = self;
                vec![$(Box::new($e),)*]
            }
        }
    };
}

all_tuples!(impl_into_encoders, 0, 15, E, e);
