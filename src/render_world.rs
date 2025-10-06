use crate::*;
use bevy::{
    ecs::entity::EntityHashMap,
    image::TextureFormatPixelInfo,
    prelude::*,
    render::{
        graph::CameraDriverLabel,
        render_asset::RenderAssets,
        render_graph::{self, NodeRunError, RenderGraph, RenderGraphContext, RenderLabel},
        render_resource::{
            Buffer, BufferDescriptor, BufferUsages, MapMode, PollType, TexelCopyBufferInfo,
            TexelCopyBufferLayout,
        },
        renderer::{RenderContext, RenderDevice},
        texture::GpuImage,
        Extract, Render, RenderApp, RenderSystems,
    },
};

pub struct CaptureRenderWorldPlugin;

impl Plugin for CaptureRenderWorldPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);

        render_app
            .init_resource::<Captures>()
            .add_systems(ExtractSchedule, extract_captures);

        let mut graph = render_app.world_mut().resource_mut::<RenderGraph>();
        graph.add_node(ImageCopy, ImageCopyDriver);
        graph.add_node_edge(CameraDriverLabel, ImageCopy);

        render_app.add_systems(Render, encode.after(RenderSystems::Render));
    }
}

#[derive(Default, Resource)]
struct Captures {
    captures: EntityHashMap<ExtractedCapture>,
}

struct ExtractedCapture {
    encoders: Encoders,
    paused: bool,
    state: Option<ExtractedCaptureState>,
}

struct ExtractedCaptureState {
    source: Handle<Image>,
    target_buffer: Buffer,
    target_image: Image,
}

impl ExtractedCaptureState {
    fn init(source: Handle<Image>, images: &Assets<Image>, render_device: &RenderDevice) -> Self {
        let source_image = images.get(&source).unwrap();
        let size = source_image.texture_descriptor.size;

        let padded_bytes_per_row =
            RenderDevice::align_copy_bytes_per_row((size.width) as usize) * 4;
        let target_buffer = render_device.create_buffer(&BufferDescriptor {
            label: None,
            size: padded_bytes_per_row as u64 * size.height as u64,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let target_image = Image::new_fill(
            size,
            TextureDimension::D2,
            &[0; 4],
            source_image.texture_descriptor.format,
            RenderAssetUsages::default(),
        );

        Self {
            source,
            target_buffer,
            target_image,
        }
    }
}

fn extract_captures(
    mut captures: ResMut<Captures>,
    captures_query: Extract<Query<(Entity, &Capture, &CaptureSource)>>,
    cameras_query: Extract<Query<&Camera>>,
    images: Extract<Res<Assets<Image>>>,
    render_device: Res<RenderDevice>,
) {
    captures.captures = captures_query
        .iter()
        .filter_map(|(entity, capture, capture_source)| match &capture.state {
            CaptureState::Idle => None,
            CaptureState::Capturing { encoders, paused } => {
                let (prev_encoder, prev_state) = match captures.captures.remove(&entity) {
                    Some(extracted) => (Some(extracted.encoders), extracted.state),
                    None => (None, None),
                };

                let encoders =
                    prev_encoder.unwrap_or_else(|| encoders.lock().unwrap().take().unwrap());

                let camera_entity = match capture_source {
                    CaptureSource::ThisCamera => entity,
                    CaptureSource::Camera(entity) => *entity,
                };
                let source = cameras_query
                    .get(camera_entity)
                    .ok()
                    .and_then(|camera| match &camera.target {
                        RenderTarget::Image(image) => Some(image.clone()),
                        _ => None,
                    });
                let source = match source {
                    Some(source) => source.handle,
                    None => {
                        return Some((
                            entity,
                            ExtractedCapture {
                                encoders,
                                paused: *paused,
                                state: None,
                            },
                        ))
                    }
                };

                let state = match prev_state {
                    Some(prev_state) if prev_state.source == source => prev_state,
                    _ => ExtractedCaptureState::init(source, &images, &render_device),
                };

                Some((
                    entity,
                    ExtractedCapture {
                        encoders,
                        paused: *paused,
                        state: Some(state),
                    },
                ))
            }
        })
        .collect();
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, RenderLabel)]
struct ImageCopy;

#[derive(Default)]
struct ImageCopyDriver;

impl render_graph::Node for ImageCopyDriver {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let captures = world.get_resource::<Captures>().unwrap();
        let gpu_images = world.get_resource::<RenderAssets<GpuImage>>().unwrap();

        for capture in captures.captures.values() {
            let capture_state = match &capture.state {
                Some(state) if !capture.paused => state,
                _ => continue,
            };

            let src_image = gpu_images.get(&capture_state.source).unwrap();

            let encoder = render_context.command_encoder();

            let block_dimensions = src_image.texture_format.block_dimensions();
            let block_size = src_image.texture_format.block_copy_size(None).unwrap();

            // Calculating correct size of image row because
            // copy_texture_to_buffer can copy image only by rows aligned wgpu::COPY_BYTES_PER_ROW_ALIGNMENT
            // That's why image in buffer can be little bit wider
            // This should be taken into account at copy from buffer stage
            let padded_bytes_per_row = RenderDevice::align_copy_bytes_per_row(
                (src_image.size.width as usize / block_dimensions.0 as usize) * block_size as usize,
            );

            let texture_extent = Extent3d {
                width: src_image.size.width,
                height: src_image.size.height,
                depth_or_array_layers: 1,
            };

            encoder.copy_texture_to_buffer(
                src_image.texture.as_image_copy(),
                TexelCopyBufferInfo {
                    buffer: &capture_state.target_buffer,
                    layout: TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(
                            std::num::NonZeroU32::new(padded_bytes_per_row as u32)
                                .unwrap()
                                .into(),
                        ),
                        rows_per_image: None,
                    },
                },
                texture_extent,
            );
        }

        Ok(())
    }
}

fn encode(mut captures: ResMut<Captures>, render_device: Res<RenderDevice>) {
    for capture in captures.captures.values_mut() {
        let capture_state = match &mut capture.state {
            Some(state) if !capture.paused => state,
            _ => continue,
        };

        // Get the data back from the gpu
        let buffer_slice = capture_state.target_buffer.slice(..);

        let (s, r) = crossbeam_channel::bounded(1);
        buffer_slice.map_async(MapMode::Read, move |r| match r {
            Ok(r) => s.send(r).expect("Failed to send map update"),
            Err(err) => panic!("Failed to map buffer {err}"),
        });
        let _ = render_device.poll(PollType::wait());
        r.recv().expect("Failed to receive the map_async message");

        let buffer_bytes = buffer_slice.get_mapped_range().to_vec();
        capture_state.target_buffer.unmap();

        // We need to ensure that this works regardless of the image dimensions
        // If the image became wider when copying from the texture to the buffer,
        // then the data is reduced to its original size when copying from the buffer to the image.
        let row_bytes = capture_state.target_image.width() as usize
            * capture_state
                .target_image
                .texture_descriptor
                .format
                .pixel_size()
                .expect("Unsupported texture format");
        let aligned_row_bytes = RenderDevice::align_copy_bytes_per_row(row_bytes);
        if row_bytes == aligned_row_bytes {
            capture_state
                .target_image
                .data
                .get_or_insert_default()
                .clone_from(&buffer_bytes);
        } else {
            // shrink data to original image size
            capture_state.target_image.data = Some(
                buffer_bytes
                    .chunks(aligned_row_bytes)
                    .take(capture_state.target_image.height() as usize)
                    .flat_map(|row| &row[..row_bytes.min(row.len())])
                    .cloned()
                    .collect(),
            );
        }

        // Call the encoder
        for encoder in &mut capture.encoders.0 {
            if let Err(err) = encoder.encode(&capture_state.target_image) {
                bevy::log::error!("Failed to encode: {:?}", err);
            }
        }
    }
}
