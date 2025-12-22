# lrle
<img width="1612" height="1034" alt="image" src="https://github.com/user-attachments/assets/315dd04e-2f87-441c-9f9d-7e6d1ccb7a1c" />

<img width="1330" height="887" alt="image" src="https://github.com/user-attachments/assets/be4ecd9a-757a-43a4-ada8-bca8214dadcb" />


> **⚠️ Work in Progress** - This is a WIP project,

A GPU-accelerated 3D terrain visualizer written in Rust. Load and explore heightmap data from FDF files with real-time camera controls and interactive visualization.

- GPU rendering via **wgpu** (cross-platform graphics)
- Interactive orbital camera with mouse and keyboard controls
- Adjustable height scaling for heightmap visualization
- Built-in egui UI panel with stats
- Efficient mesh generation from heightmap grids

## Building

```bash
cargo build --release
```

## Usage

```bash
lrle terrain.fdf
lrle terrain.fdf --height-scale 2.0
```
