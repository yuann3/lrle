mod renderer;
mod terrain;

use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use renderer::Renderer;
use terrain::{TerrainMesh, load_fdf};

#[derive(Parser, Debug)]
#[command(name = "lrle")]
#[command(about = "Modern terrain visualization tool")]
struct Args {
    /// Path to .fdf file to load
    file: String,

    /// Height scale multiplier
    #[arg(long, default_value = "1.0")]
    height_scale: f32,
}

struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    mesh: TerrainMesh,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attrs = Window::default_attributes().with_title("lrle - Terrain Viewer");
            let window = Arc::new(event_loop.create_window(window_attrs).unwrap());

            let renderer = pollster::block_on(Renderer::new(window.clone())).unwrap();

            self.window = Some(window);
            self.renderer = Some(renderer);

            // Upload mesh to GPU
            if let Some(ref mut renderer) = self.renderer {
                renderer.upload_mesh(&self.mesh);
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
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
            WindowEvent::Resized(physical_size) => {
                if let Some(ref mut renderer) = self.renderer {
                    renderer.resize(physical_size);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(ref mut renderer) = self.renderer {
                    match renderer.render() {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => renderer.resize(renderer.size),
                        Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                        Err(e) => eprintln!("Render error: {:?}", e),
                    }
                }
                if let Some(ref window) = self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();

    // Load terrain
    let terrain_data = load_fdf(&args.file)?;
    println!(
        "Loaded terrain: {}x{}, height range: {:?}",
        terrain_data.width,
        terrain_data.height,
        terrain_data.height_bounds()
    );

    // Generate mesh
    let mesh = TerrainMesh::from_terrain(&terrain_data, args.height_scale);
    println!(
        "Generated mesh: {} vertices, {} indices",
        mesh.vertices.len(),
        mesh.indices.len()
    );

    // Create window and run
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App {
        window: None,
        renderer: None,
        mesh,
    };

    event_loop.run_app(&mut app)?;

    Ok(())
}
