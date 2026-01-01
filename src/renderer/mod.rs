//! GPU rendering pipeline using wgpu.
//!
//! This module provides the [`Renderer`] struct which handles:
//! - wgpu device and surface initialization
//! - Shader compilation and pipeline setup
//! - Mesh upload and rendering
//! - Camera uniform updates

pub mod camera;

use std::sync::Arc;
use std::time::Instant;

use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::terrain::{ColorScheme, TerrainMesh, Vertex};
use crate::ui::Ui;
use camera::Camera;
pub use camera::Projection;

/// Rendering mode for the terrain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderMode {
    /// Wireframe rendering (lines only)
    Wireframe,
    /// Solid shaded rendering with lighting
    #[default]
    Solid,
    /// Both wireframe and solid overlaid
    Both,
}

/// Lighting configuration for solid rendering.
#[derive(Debug, Clone, Copy)]
pub struct LightingConfig {
    /// Light direction (normalized, pointing toward light)
    pub direction: Vec3,
    /// Light color/intensity
    pub color: Vec3,
    /// Ambient light strength (0.0 - 1.0)
    pub ambient: f32,
}

impl Default for LightingConfig {
    fn default() -> Self {
        Self {
            // Default: light from upper-right-front
            direction: Vec3::new(0.5, 0.8, 0.3).normalize(),
            color: Vec3::ONE,
            ambient: 0.3,
        }
    }
}

/// Uniform data sent to shaders (wireframe - simple).
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct WireframeUniforms {
    view_proj: [[f32; 4]; 4],
}

impl WireframeUniforms {
    fn new() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        }
    }

    fn update(&mut self, camera: &Camera, aspect: f32) {
        self.view_proj = camera
            .build_view_projection_matrix(aspect)
            .to_cols_array_2d();
    }
}

/// Uniform data for solid shaded rendering with lighting.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct SolidUniforms {
    view_proj: [[f32; 4]; 4],
    light_dir: [f32; 3],
    _pad0: f32,
    light_color: [f32; 3],
    ambient: f32,
}

impl SolidUniforms {
    fn new() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            light_dir: [0.5, 0.8, 0.3],
            _pad0: 0.0,
            light_color: [1.0, 1.0, 1.0],
            ambient: 0.3,
        }
    }

    fn update(&mut self, camera: &Camera, aspect: f32, lighting: &LightingConfig) {
        self.view_proj = camera
            .build_view_projection_matrix(aspect)
            .to_cols_array_2d();
        self.light_dir = lighting.direction.to_array();
        self.light_color = lighting.color.to_array();
        self.ambient = lighting.ambient;
    }
}

/// GPU renderer managing wgpu state and rendering.
///
/// Handles the complete rendering pipeline from mesh upload to frame presentation.
pub struct Renderer {
    // Core wgpu objects
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    /// Current window size (for aspect ratio and resize handling)
    pub size: winit::dpi::PhysicalSize<u32>,

    // Depth buffer
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,

    // Wireframe pipeline
    wireframe_pipeline: wgpu::RenderPipeline,
    wireframe_uniform_buffer: wgpu::Buffer,
    wireframe_bind_group: wgpu::BindGroup,

    // Solid pipeline
    solid_pipeline: wgpu::RenderPipeline,
    solid_uniform_buffer: wgpu::Buffer,
    solid_bind_group: wgpu::BindGroup,

    // Mesh buffers
    vertex_buffer: Option<wgpu::Buffer>,
    wireframe_index_buffer: Option<wgpu::Buffer>,
    triangle_index_buffer: Option<wgpu::Buffer>,
    num_wireframe_indices: u32,
    num_triangle_indices: u32,

    /// Current render mode
    pub render_mode: RenderMode,

    /// Lighting configuration
    pub lighting: LightingConfig,

    /// Color scheme for terrain
    pub color_scheme: ColorScheme,

    /// Orbital camera for viewing the terrain
    pub camera: Camera,

    // egui
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,

    /// UI state
    pub ui: Ui,

    /// Frame_time for FPS calculation
    last_frame: Instant,
    frame_count: u32,
    fps: f32,

    /// Terrain data for mesh regeneration
    terrain_data: Option<crate::terrain::TerrainData>,
    /// Height scale for mesh regeneration
    height_scale: f32,
    /// Previous color scheme to detect changes
    prev_color_scheme: ColorScheme,
}

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

fn create_depth_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Depth Texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

impl Renderer {
    /// Create a new renderer for the given window.
    ///
    /// # Arguments
    ///
    /// * `window` - The window to render to
    ///
    /// # Errors
    ///
    /// Returns an error if GPU initialization fails.
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();

        // Create wgpu instance
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // Create surface for the window
        let surface = instance.create_surface(window.clone())?;

        // Request GPU adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        // Create device and queue
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                label: None,
                memory_hints: Default::default(),
                trace: Default::default(),
                experimental_features: Default::default(),
            })
            .await?;

        // Configure surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Init egui
        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx,
            egui::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            Some(2048),
        );
        let egui_renderer = egui_wgpu::Renderer::new(
            &device,
            surface_format,
            egui_wgpu::RendererOptions {
                depth_stencil_format: Some(DEPTH_FORMAT),
                ..Default::default()
            },
        );
        let ui = Ui::new();

        // Create depth texture
        let (depth_texture, depth_view) = create_depth_texture(&device, size.width, size.height);

        // Load wireframe shader
        let wireframe_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Wireframe Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/terrain.wgsl").into()),
        });

        // Load solid shader
        let solid_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Solid Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/solid.wgsl").into()),
        });

        // Create wireframe uniform buffer and bind group
        let wireframe_uniforms = WireframeUniforms::new();
        let wireframe_uniform_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Wireframe Uniform Buffer"),
                contents: bytemuck::cast_slice(&[wireframe_uniforms]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let wireframe_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                label: Some("Wireframe Bind Group Layout"),
            });

        let wireframe_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &wireframe_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wireframe_uniform_buffer.as_entire_binding(),
            }],
            label: Some("Wireframe Bind Group"),
        });

        // Create solid uniform buffer and bind group
        let solid_uniforms = SolidUniforms::new();
        let solid_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Solid Uniform Buffer"),
            contents: bytemuck::cast_slice(&[solid_uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let solid_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("Solid Bind Group Layout"),
            });

        let solid_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &solid_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: solid_uniform_buffer.as_entire_binding(),
            }],
            label: Some("Solid Bind Group"),
        });

        // Create wireframe pipeline
        let wireframe_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Wireframe Pipeline Layout"),
                bind_group_layouts: &[&wireframe_bind_group_layout],
                push_constant_ranges: &[],
            });

        let wireframe_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Wireframe Pipeline"),
            layout: Some(&wireframe_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &wireframe_shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &wireframe_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        // Create solid pipeline
        let solid_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Solid Pipeline Layout"),
                bind_group_layouts: &[&solid_bind_group_layout],
                push_constant_ranges: &[],
            });

        let solid_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Solid Pipeline"),
            layout: Some(&solid_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &solid_shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &solid_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let camera = Camera::new();

        Ok(Self {
            surface,
            device,
            queue,
            config,
            size,
            depth_texture,
            depth_view,
            wireframe_pipeline,
            wireframe_uniform_buffer,
            wireframe_bind_group,
            solid_pipeline,
            solid_uniform_buffer,
            solid_bind_group,
            vertex_buffer: None,
            wireframe_index_buffer: None,
            triangle_index_buffer: None,
            num_wireframe_indices: 0,
            num_triangle_indices: 0,
            render_mode: RenderMode::default(),
            lighting: LightingConfig::default(),
            color_scheme: ColorScheme::default(),
            camera,
            egui_state,
            egui_renderer,
            ui,
            last_frame: Instant::now(),
            frame_count: 0,
            fps: 0.0,
            terrain_data: None,
            height_scale: 1.0,
            prev_color_scheme: ColorScheme::default(),
        })
    }

    /// Handle window event
    pub fn handle_window_event(
        &mut self,
        window: &Window,
        event: &winit::event::WindowEvent,
    ) -> bool {
        self.egui_state.on_window_event(window, event).consumed
    }

    /// Handle window resize.
    ///
    /// Reconfigures the surface and depth buffer for the new size.
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            // Recreate depth texture for new size
            let (depth_texture, depth_view) =
                create_depth_texture(&self.device, new_size.width, new_size.height);
            self.depth_texture = depth_texture;
            self.depth_view = depth_view;
        }
    }

    /// Upload terrain data to GPU.
    ///
    /// Stores the terrain data and generates a mesh with the current color scheme.
    /// The terrain data is retained so the mesh can be regenerated when the color scheme changes.
    pub fn upload_terrain(&mut self, terrain: &crate::terrain::TerrainData, height_scale: f32) {
        self.terrain_data = Some(terrain.clone());
        self.height_scale = height_scale;
        self.regenerate_mesh();
    }

    /// Regenerate mesh from stored terrain data with current color scheme.
    fn regenerate_mesh(&mut self) {
        if let Some(ref terrain) = self.terrain_data {
            let mesh = TerrainMesh::from_terrain_with_options(
                terrain,
                self.height_scale,
                crate::terrain::mesh::ShadingMode::Smooth,
                self.color_scheme,
            );
            self.upload_mesh_buffers(&mesh);
            self.prev_color_scheme = self.color_scheme;
        }
    }

    /// Upload terrain mesh to GPU buffers.
    ///
    /// Creates vertex and index buffers for both wireframe and solid rendering.
    fn upload_mesh_buffers(&mut self, mesh: &TerrainMesh) {
        if mesh.vertices.is_empty() {
            self.vertex_buffer = None;
            self.wireframe_index_buffer = None;
            self.triangle_index_buffer = None;
            self.num_wireframe_indices = 0;
            self.num_triangle_indices = 0;
            return;
        }

        self.vertex_buffer = Some(self.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&mesh.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            },
        ));

        self.wireframe_index_buffer = Some(self.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Wireframe Index Buffer"),
                contents: bytemuck::cast_slice(&mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            },
        ));

        self.triangle_index_buffer = Some(self.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Triangle Index Buffer"),
                contents: bytemuck::cast_slice(&mesh.triangle_indices),
                usage: wgpu::BufferUsages::INDEX,
            },
        ));

        self.num_wireframe_indices = mesh.indices.len() as u32;
        self.num_triangle_indices = mesh.triangle_indices.len() as u32;
    }

    /// Render a frame.
    ///
    /// Updates camera uniforms and draws the terrain based on current render mode.
    ///
    /// # Errors
    ///
    /// Returns [`wgpu::SurfaceError`] if surface acquisition fails.
    pub fn render(&mut self, window: &Window) -> Result<(), wgpu::SurfaceError> {
        // Update FPS counter
        self.frame_count += 1;
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame).as_secs_f32();
        if elapsed >= 1.0 {
            self.fps = self.frame_count as f32 / elapsed;
            self.frame_count = 0;
            self.last_frame = now;
        }

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Update uniforms
        let aspect = self.size.width as f32 / self.size.height as f32;

        // Update wireframe uniforms
        let mut wireframe_uniforms = WireframeUniforms::new();
        wireframe_uniforms.update(&self.camera, aspect);
        self.queue.write_buffer(
            &self.wireframe_uniform_buffer,
            0,
            bytemuck::cast_slice(&[wireframe_uniforms]),
        );

        // Update solid uniforms
        let mut solid_uniforms = SolidUniforms::new();
        solid_uniforms.update(&self.camera, aspect, &self.lighting);
        self.queue.write_buffer(
            &self.solid_uniform_buffer,
            0,
            bytemuck::cast_slice(&[solid_uniforms]),
        );

        // Begin egui frame
        let raw_input = self.egui_state.take_egui_input(window);
        let egui_ctx = self.egui_state.egui_ctx().clone();
        let full_output = egui_ctx.run(raw_input, |ctx| {
            let response = self.ui.render(
                ctx,
                &mut self.camera,
                &mut self.render_mode,
                &mut self.color_scheme,
                &mut self.lighting,
                self.fps,
            );
            if response.reset_camera {
                self.camera = Camera::new();
            }
        });

        // Regenerate mesh if color scheme changed
        if self.color_scheme != self.prev_color_scheme {
            self.regenerate_mesh();
        }

        // Handle egui platform output (cursor changes, etc.)
        self.egui_state
            .handle_platform_output(window, full_output.platform_output);

        // Prepare egui for rendering
        let paint_jobs = egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.size.width, self.size.height],
            pixels_per_point: full_output.pixels_per_point,
        };

        // Update egui textures
        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer
                .update_texture(&self.device, &self.queue, *id, image_delta);
        }

        // Create command encoder
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Upload egui buffers
        self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        // Begin render pass
        {
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.15,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // Convert to 'static lifetime for egui compatibility
            let mut render_pass = render_pass.forget_lifetime();

            // Draw terrain based on render mode
            if let Some(vertex_buffer) = &self.vertex_buffer {
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));

                // Draw solid first (if applicable)
                if matches!(self.render_mode, RenderMode::Solid | RenderMode::Both) {
                    if let Some(triangle_index_buffer) = &self.triangle_index_buffer {
                        render_pass.set_pipeline(&self.solid_pipeline);
                        render_pass.set_bind_group(0, &self.solid_bind_group, &[]);
                        render_pass.set_index_buffer(
                            triangle_index_buffer.slice(..),
                            wgpu::IndexFormat::Uint32,
                        );
                        render_pass.draw_indexed(0..self.num_triangle_indices, 0, 0..1);
                    }
                }

                // Draw wireframe on top (if applicable)
                if matches!(self.render_mode, RenderMode::Wireframe | RenderMode::Both) {
                    if let Some(wireframe_index_buffer) = &self.wireframe_index_buffer {
                        render_pass.set_pipeline(&self.wireframe_pipeline);
                        render_pass.set_bind_group(0, &self.wireframe_bind_group, &[]);
                        render_pass.set_index_buffer(
                            wireframe_index_buffer.slice(..),
                            wgpu::IndexFormat::Uint32,
                        );
                        render_pass.draw_indexed(0..self.num_wireframe_indices, 0, 0..1);
                    }
                }
            }

            // Render egui UI
            self.egui_renderer
                .render(&mut render_pass, &paint_jobs, &screen_descriptor);
        }

        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        // Submit commands and present
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
