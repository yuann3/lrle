//! # lrle - A Terrain Visualization Tool
//!
//! A GPU terrain viewer built in wgpu
//!
//! ## Usage
//!
//! ```bash
//! lrle terrain.fdf                    # Load file with defaults
//! lrle terrain.fdf --height-scale 2.0 # Load with height multiplier
//! ```
//!
//! ## Controls
//!
//! - `ESC` - Quit application
//! - Left Drag: Rotate camera
//! - Scroll: Zoom in/out
//! - Shift+Drag / Middle Drag: Pan
//! - R: Reset camera
//! - Tab: Toggle UI panel
//! - ESC: Quit


mod input;
mod renderer;
mod terrain;
mod ui;

use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use input::InputController;
use renderer::Renderer;
use terrain::{load_fdf, TerrainMesh};

/// Command-line arguments for lrle
#[derive(Parser, Debug)]
#[command(name = "lrle")]
#[command(version, about = "Modern terrain visualization tool", long_about = None)]
struct Args {
    /// Path to .fdf file to load
    file: String,

    /// Height scale multiplier (default: 1.0)
    #[arg(long, default_value = "1.0")]
    height_scale: f32,
}

/// Main application state managing window, renderer, and terrain mesh.
struct App {
    /// The application window (created on resume)
    window: Option<Arc<Window>>,
    /// GPU renderer instance
    renderer: Option<Renderer>,
    /// Pre-generated terrain mesh to upload to GPU
    mesh: TerrainMesh,
    /// Input controller for camera
    input: InputController
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Only create window once
        if self.window.is_some() {
            return;
        }

        let window_attrs = Window::default_attributes().with_title("lrle - Terrain Viewer");

        let window = match event_loop.create_window(window_attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                log::error!("Failed to create window: {}", e);
                event_loop.exit();
                return;
            }
        };

        match pollster::block_on(Renderer::new(window.clone())) {
            Ok(mut renderer) => {
                renderer.upload_mesh(&self.mesh);
                self.renderer = Some(renderer);
                self.window = Some(window);
            }
            Err(e) => {
                log::error!("Failed to create renderer: {}", e);
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        // Let egui handle the event first
        if let Some(ref mut renderer) = self.renderer {
            if let Some(ref window) = self.window {
                if renderer.handle_window_event(window, &event) {
                    return; // egui consumed the event
                }
            }
        }

        match event {
            // Close on window close button or ESC key
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                event_loop.exit();
            }

            // Handle other keyboard input for camera control
            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    physical_key: PhysicalKey::Code(key),
                    state,
                    ..
                },
                ..
            } => {
                if let Some(ref mut renderer) = self.renderer {
                    self.input.handle_keyboard(key, state, &mut renderer.camera);
                }
            }

            // Mouse button events
            WindowEvent::MouseInput { button, state, .. } => {
                self.input.handle_mouse_button(button, state);
            }

            // Mouse movement
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(ref mut renderer) = self.renderer {
                    self.input.handle_mouse_move(
                        position.x as f32,
                        position.y as f32,
                        &mut renderer.camera,
                    );
                }
            }

            // Mouse scroll for zoom
            WindowEvent::MouseWheel { delta, .. } => {
                if let Some(ref mut renderer) = self.renderer {
                    self.input.handle_scroll(delta, &mut renderer.camera);
                }
            }


            // Handle window resize
            WindowEvent::Resized(physical_size) => {
                if let Some(ref mut renderer) = self.renderer {
                    renderer.resize(physical_size);
                }
            }

            // Render frame
            WindowEvent::RedrawRequested => {
                if let (Some(ref mut renderer), Some(ref window)) = (&mut self.renderer, &self.window) {
                    match renderer.render(window) {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => {
                            renderer.resize(renderer.size);
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            log::error!("Out of GPU memory");
                            event_loop.exit();
                        }
                        Err(e) => {
                            log::warn!("Render error: {:?}", e);
                        }
                    }
                }

                // Request next frame
                if let Some(ref window) = self.window {
                    window.request_redraw();
                }
            }

            _ => {}
        }
    }
}

fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    let args = Args::parse();

    // Load terrain data from file
    let terrain_data = load_fdf(&args.file)?;
    log::info!(
        "Loaded terrain: {}x{}, height range: {:?}",
        terrain_data.width,
        terrain_data.height,
        terrain_data.height_bounds()
    );

    // Generate mesh from terrain data
    let mesh = TerrainMesh::from_terrain(&terrain_data, args.height_scale);
    log::info!(
        "Generated mesh: {} vertices, {} indices",
        mesh.vertices.len(),
        mesh.indices.len()
    );

    // Create event loop and run application
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App {
        window: None,
        renderer: None,
        mesh,
        input: InputController::new(),
    };

    event_loop.run_app(&mut app)?;

    Ok(())
}
