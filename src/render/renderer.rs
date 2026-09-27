//! The instanced sprite renderer.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::error::{Result, SagaError};
use crate::geometry::{Point, Rect};
use crate::render::font;

/// Handle of a texture owned by the renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureId(pub usize);

/// A linear RGBA color with components in `0.0..=1.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color(pub [f32; 4]);

impl Color {
    pub const WHITE: Color = Color([1.0, 1.0, 1.0, 1.0]);
    pub const BLACK: Color = Color([0.0, 0.0, 0.0, 1.0]);

    pub const fn rgba(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Color([red, green, blue, alpha])
    }

    pub const fn rgb(red: f32, green: f32, blue: f32) -> Self {
        Color([red, green, blue, 1.0])
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    screen: [f32; 2],
    offset: [f32; 2],
    scale: [f32; 2],
    padding: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Instance {
    position: [f32; 2],
    size: [f32; 2],
    uv_offset: [f32; 2],
    uv_size: [f32; 2],
    color: [f32; 4],
}

struct Texture {
    bind_group: wgpu::BindGroup,
    width: u32,
    height: u32,
}

/// A GPU accelerated 2D renderer working in virtual pixel coordinates.
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    quad_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,
    textures: Vec<Texture>,
    texture_cache: HashMap<String, TextureId>,
    instances: Vec<(TextureId, Instance)>,
    white: TextureId,
    font: TextureId,
    virtual_size: (u32, u32),
    clear_color: Color,
}

impl Renderer {
    /// Creates a renderer drawing into `window`.
    pub fn new(window: Arc<Window>, virtual_size: (u32, u32)) -> Result<Self> {
        pollster::block_on(Self::new_async(window, virtual_size))
    }

    async fn new_async(window: Arc<Window>, virtual_size: (u32, u32)) -> Result<Self> {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let surface = instance
            .create_surface(window)
            .map_err(|error| SagaError::Graphics(format!("cannot create a surface: {error}")))?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| SagaError::Graphics("no suitable GPU adapter found".to_string()))?;
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("saga device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults()
                        .using_resolution(adapter.limits()),
                },
                None,
            )
            .await
            .map_err(|error| SagaError::Graphics(format!("cannot create a device: {error}")))?;

        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(|format| format.is_srgb())
            .unwrap_or(capabilities.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: capabilities
                .present_modes
                .iter()
                .copied()
                .find(|mode| *mode == wgpu::PresentMode::Mailbox)
                .unwrap_or(wgpu::PresentMode::Fifo),
            alpha_mode: capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("saga sprite shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("saga uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("saga uniform layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("saga uniforms"),
            layout: &uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("saga texture layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("saga pipeline layout"),
            bind_group_layouts: &[&uniform_layout, &texture_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("saga sprite pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: (std::mem::size_of::<f32>() * 2) as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Instance>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![
                            1 => Float32x2,
                            2 => Float32x2,
                            3 => Float32x2,
                            4 => Float32x2,
                            5 => Float32x4
                        ],
                    },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        const QUAD: [[f32; 2]; 6] = [
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 1.0],
            [0.0, 0.0],
            [1.0, 1.0],
            [0.0, 1.0],
        ];
        let quad_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("saga quad"),
            contents: bytemuck::cast_slice(&QUAD),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let instance_capacity = 1024;
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("saga instances"),
            size: (instance_capacity * std::mem::size_of::<Instance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("saga sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let mut renderer = Self {
            surface,
            device,
            queue,
            config,
            pipeline,
            uniform_buffer,
            uniform_bind_group,
            texture_layout,
            sampler,
            quad_buffer,
            instance_buffer,
            instance_capacity,
            textures: Vec::new(),
            texture_cache: HashMap::new(),
            instances: Vec::new(),
            white: TextureId(0),
            font: TextureId(0),
            virtual_size: (virtual_size.0.max(1), virtual_size.1.max(1)),
            clear_color: Color::BLACK,
        };

        renderer.white = renderer.create_texture(1, 1, &[255, 255, 255, 255]);
        let (font_width, font_height, font_pixels) = font::atlas_rgba();
        renderer.font = renderer.create_texture(font_width, font_height, &font_pixels);
        Ok(renderer)
    }

    /// Virtual resolution every draw call refers to.
    pub fn virtual_size(&self) -> (u32, u32) {
        self.virtual_size
    }

    pub fn white_texture(&self) -> TextureId {
        self.white
    }

    pub fn set_clear_color(&mut self, color: Color) {
        self.clear_color = color;
    }

    pub fn texture_size(&self, texture: TextureId) -> (u32, u32) {
        self.textures
            .get(texture.0)
            .map(|texture| (texture.width, texture.height))
            .unwrap_or((1, 1))
    }

    /// Reconfigures the surface after the window was resized.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    /// Loads a PNG file, reusing the texture when the same path is requested twice.
    pub fn load_texture(&mut self, path: &Path) -> Result<TextureId> {
        let key = path.to_string_lossy().to_string();
        if let Some(texture) = self.texture_cache.get(&key) {
            return Ok(*texture);
        }
        let bytes = std::fs::read(path).map_err(|source| SagaError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let image = image::load_from_memory(&bytes)
            .map_err(|error| SagaError::Asset(format!("cannot decode `{key}`: {error}")))?
            .to_rgba8();
        let texture = self.create_texture(image.width(), image.height(), &image);
        self.texture_cache.insert(key, texture);
        Ok(texture)
    }

    /// Uploads raw RGBA8 pixels as a new texture.
    pub fn create_texture(&mut self, width: u32, height: u32, pixels: &[u8]) -> TextureId {
        let width = width.max(1);
        let height = height.max(1);
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("saga texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            pixels,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("saga texture"),
            layout: &self.texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        self.textures.push(Texture {
            bind_group,
            width,
            height,
        });
        TextureId(self.textures.len() - 1)
    }

    /// Starts a new frame; every queued draw command is discarded.
    pub fn begin_frame(&mut self) {
        self.instances.clear();
    }

    /// Draws a sub-rectangle of a texture (in pixels) into `destination`.
    pub fn draw_sprite(
        &mut self,
        texture: TextureId,
        destination: Rect,
        source: Option<Rect>,
        color: Color,
    ) {
        let (width, height) = self.texture_size(texture);
        let source = source.unwrap_or(Rect::new(0.0, 0.0, width as f32, height as f32));
        self.instances.push((
            texture,
            Instance {
                position: [destination.x, destination.y],
                size: [destination.width, destination.height],
                uv_offset: [source.x / width as f32, source.y / height as f32],
                uv_size: [source.width / width as f32, source.height / height as f32],
                color: color.0,
            },
        ));
    }

    /// Draws a solid rectangle.
    pub fn draw_rect(&mut self, rect: Rect, color: Color) {
        let white = self.white;
        self.draw_sprite(white, rect, None, color);
    }

    /// Draws `text` with its top left corner at `position`.
    pub fn draw_text(&mut self, text: &str, position: Point, scale: f32, color: Color) {
        let font_texture = self.font;
        let mut pen_x = position.x;
        for character in text.chars() {
            if let Some(index) = font::glyph_index(character) {
                let source = Rect::new(
                    (index * font::GLYPH_WIDTH) as f32,
                    0.0,
                    font::GLYPH_WIDTH as f32,
                    font::GLYPH_HEIGHT as f32,
                );
                let destination = Rect::new(
                    pen_x,
                    position.y,
                    font::GLYPH_WIDTH as f32 * scale,
                    font::GLYPH_HEIGHT as f32 * scale,
                );
                self.draw_sprite(font_texture, destination, Some(source), color);
            }
            pen_x += font::GLYPH_ADVANCE as f32 * scale;
        }
    }

    /// Width in virtual pixels of `text` drawn at `scale`.
    pub fn text_width(text: &str, scale: f32) -> f32 {
        font::text_width(text) as f32 * scale
    }

    /// Height in virtual pixels of a line of text drawn at `scale`.
    pub fn text_height(scale: f32) -> f32 {
        font::GLYPH_HEIGHT as f32 * scale
    }

    /// Letterboxed placement of the virtual viewport inside the window.
    fn viewport(&self) -> (f32, f32, f32) {
        let scale = (self.config.width as f32 / self.virtual_size.0 as f32)
            .min(self.config.height as f32 / self.virtual_size.1 as f32)
            .max(f32::MIN_POSITIVE);
        let width = self.virtual_size.0 as f32 * scale;
        let height = self.virtual_size.1 as f32 * scale;
        (
            (self.config.width as f32 - width) / 2.0,
            (self.config.height as f32 - height) / 2.0,
            scale,
        )
    }

    /// Converts physical window coordinates into virtual game coordinates.
    pub fn window_to_virtual(&self, x: f32, y: f32) -> Point {
        let (offset_x, offset_y, scale) = self.viewport();
        Point::new((x - offset_x) / scale, (y - offset_y) / scale)
    }

    /// Submits every queued draw command.
    pub fn render(&mut self) -> Result<()> {
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            Err(error) => {
                return Err(SagaError::Graphics(format!(
                    "cannot acquire a frame: {error}"
                )))
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let (offset_x, offset_y, scale) = self.viewport();
        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::bytes_of(&Uniforms {
                screen: [self.config.width as f32, self.config.height as f32],
                offset: [offset_x, offset_y],
                scale: [scale, scale],
                padding: [0.0, 0.0],
            }),
        );

        if self.instances.len() > self.instance_capacity {
            self.instance_capacity = self.instances.len().next_power_of_two();
            self.instance_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("saga instances"),
                size: (self.instance_capacity * std::mem::size_of::<Instance>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        let raw: Vec<Instance> = self
            .instances
            .iter()
            .map(|(_, instance)| *instance)
            .collect();
        if !raw.is_empty() {
            self.queue
                .write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&raw));
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("saga encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("saga pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: self.clear_color.0[0] as f64,
                            g: self.clear_color.0[1] as f64,
                            b: self.clear_color.0[2] as f64,
                            a: self.clear_color.0[3] as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            pass.set_vertex_buffer(0, self.quad_buffer.slice(..));
            pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

            // Draw contiguous runs of instances sharing the same texture.
            let mut start = 0usize;
            while start < self.instances.len() {
                let texture = self.instances[start].0;
                let mut end = start + 1;
                while end < self.instances.len() && self.instances[end].0 == texture {
                    end += 1;
                }
                if let Some(texture) = self.textures.get(texture.0) {
                    pass.set_bind_group(1, &texture.bind_group, &[]);
                    pass.draw(0..6, start as u32..end as u32);
                }
                start = end;
            }
        }
        self.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }
}
