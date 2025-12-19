# lrle - Modern Terrain Visualization Tool

**Date:** 2025-12-19
**Status:** Design Complete

## Overview

**lrle** is a modern terrain visualization tool built in Rust, reimagining the classic FDF (fil de fer) wireframe project with GPU-accelerated rendering and flexible viewing modes. The goal is to create a beautiful, performant tool for visualizing height-mapped terrain data with multiple rendering styles and projections.

## Design Decisions Summary

- **Rendering:** wgpu (native Metal/Vulkan/DX12) for GPU-accelerated graphics
- **UI Framework:** egui (immediate mode, integrates well with wgpu)
- **Math Library:** glam (fast, SIMD-optimized for graphics)
- **Camera:** Orbital camera with perspective/orthographic projections
- **Input Formats:** .fdf files + procedural generation (Perlin/Simplex noise)
- **Performance:** Adaptive strategy - simple for small terrains, chunking + frustum culling for large (8000x8000+)
- **Interaction:** CLI flags for initial settings + real-time UI controls

## Architecture

### Project Structure

```
lrle/
├── Cargo.toml
├── README.md
├── .gitignore
├── examples/          # Sample .fdf files
│   └── 42.fdf
├── src/
│   ├── main.rs              # CLI parsing, window setup
│   ├── app.rs               # Main application state
│   ├── terrain/             # Terrain data structures
│   │   ├── mod.rs
│   │   ├── loader.rs        # .fdf file parsing
│   │   ├── generator.rs     # Procedural terrain (noise)
│   │   └── mesh.rs          # Height data → 3D mesh conversion
│   ├── renderer/            # wgpu rendering pipeline
│   │   ├── mod.rs
│   │   ├── pipeline.rs      # Shader setup, render passes
│   │   ├── camera.rs        # Orbital camera + projections
│   │   └── styles.rs        # Rendering modes & color schemes
│   ├── ui/                  # egui interface
│   │   ├── mod.rs
│   │   └── controls.rs      # UI panels
│   └── utils/
│       └── mod.rs
└── shaders/                 # WGSL shaders
    ├── terrain.wgsl         # Main terrain shader
    └── wireframe.wgsl       # Wireframe rendering
```

### Data Flow

1. **Input:** CLI args → load .fdf file OR generate procedural terrain
2. **Parse:** Height data → TerrainData structure
3. **Mesh:** Build 3D mesh (vertices, indices, normals)
4. **Upload:** Transfer to GPU (vertex/index buffers)
5. **Render Loop:**
   - Update camera transform
   - Apply projection matrix
   - Execute shaders
   - Render to screen
   - Overlay egui UI

## Core Systems

### 1. Terrain Data

**TerrainData Structure:**
```rust
struct TerrainData {
    width: usize,
    height: usize,
    points: Vec<Vec<f32>>,              // 2D grid of heights
    colors: Option<Vec<Vec<Color>>>,    // Optional vertex colors
    bounds: (f32, f32),                 // min/max height
}
```

**Supported Formats:**

1. **.fdf files** (original format):
   ```
   0 0 1 2 3
   0 1 2 3 4
   ```
   - Space-separated numbers (heights)
   - Optional colors: `10,0xFF0000` (height,color)

2. **Procedural generation:**
   - Perlin noise, Simplex noise, Diamond-square
   - Parameters: size, seed, frequency, octaves
   - CLI: `lrle --generate 512x512`

**Mesh Generation:**
- Convert 2D grid → 3D vertices: `(x, height[x][y] * scale, y)`
- Generate triangle indices (2 triangles per quad)
- Calculate normals (smooth or flat, switchable)
- Center mesh at origin for orbital camera

### 2. Rendering System

**Rendering Modes:**
1. Wireframe - Classic line-based mesh
2. Solid Shaded - Filled triangles with lighting
3. Wireframe + Solid - Overlay both
4. Contour Lines - Elevation-based lines (topographic)
5. Point Cloud - Just vertices

**Color Schemes:**
1. Terrain - Height gradient: blue → green → brown → white
2. Heatmap - Red-yellow-blue scientific visualization
3. Monochrome - Single color + lighting
4. Custom Gradient - User-defined color stops
5. Vertex Colors - From .fdf color data (if present)

**Lighting:**
- Directional light (simulated sun)
- Adjustable: direction, intensity, ambient strength
- Future: ambient occlusion for depth

**Shader Strategy:**
- Uber-shader with uniforms for mode/color selection
- Uniform buffer: camera matrices, light params, color data, render flags
- Vertex shader: height scaling, transforms
- Fragment shader: lighting, height-based color mapping

### 3. Camera System

**Orbital Camera:**
- Rotates around terrain center
- Parameters:
  - Distance (zoom)
  - Azimuth (horizontal rotation)
  - Elevation (vertical rotation, clamped)
  - Target point (look-at center)

**Projection Modes:**
1. **Perspective** - Natural 3D view, configurable FOV (~60°)
2. **Orthographic** - Parallel projection, no perspective distortion

**Isometric Preset:**
- "I" key → animate to isometric view
- Orthographic + azimuth 45° + elevation 35.264° (arctan(1/√2))
- Smooth camera animation

**Controls:**

*Mouse:*
- Left drag → rotate (azimuth/elevation)
- Scroll → zoom in/out
- Middle drag (or Shift+Left) → pan target

*Keyboard:*
- `I` → Snap to isometric view
- `P` → Toggle perspective/orthographic
- `R` → Reset camera
- `ESC` → Quit
- Arrow keys → alternative rotation

*UI:*
- Manual angle sliders
- FOV slider (perspective mode)
- Preset buttons (Isometric/Top/Side/Front/Reset)

### 4. User Interface (egui)

**Left Sidebar Panel** (collapsible with Tab):

**File Section:**
- Current file display
- Load File / Generate Terrain buttons
- Terrain info: dimensions, points, height range

**Rendering Section:**
- Mode dropdown: Wireframe/Solid/Combo/Contour/Points
- Color scheme dropdown
- Custom gradient editor (color pickers)
- Wireframe thickness slider
- Smooth/Flat normals toggle

**Lighting Section:**
- Direction sliders (azimuth, elevation)
- Intensity slider
- Ambient strength slider
- Reset button

**Camera Section:**
- Projection toggle (Perspective/Orthographic)
- FOV slider (perspective only)
- Position display (read-only)
- Preset buttons

**Terrain Section:**
- Height scale multiplier slider
- Show grid toggle
- Show normals toggle (debug)

**Performance Section:**
- FPS counter
- Vertex/triangle count
- Chunk statistics

**Status Bar** (bottom):
- Current mode/projection
- File path
- Control hints

### 5. Performance Strategy

**Adaptive Approach Based on Terrain Size:**

- **Small (<1000²):** Render entire mesh, simple and fast
- **Medium (1000² - 4000²):** Add frustum culling only
- **Large (>4000²):** Chunking + frustum culling

**Chunking System (for 8000x8000+):**
- Divide terrain into chunks (e.g., 256x256)
- Separate vertex/index buffers per chunk
- Render only visible chunks (frustum culling)
- ~250 chunks for 8000², render ~20-50 at once

**Memory Optimization:**
- Keep original data for re-meshing (height scale changes)
- Consider f16 positions if memory becomes issue
- Use `rayon` for parallel chunk processing

### 6. Error Handling

**File Loading:**
- File not found: "Cannot open file: [path]"
- Parse errors: "Parse error at line 5: expected number"
- Inconsistent rows: "Row 3 has 10 values, expected 19"
- Empty file warning
- Size limit: "Terrain is 10000x10000. Consider downsampling."

**Runtime:**
- GPU errors: graceful messages, no crashes
- Out of memory: helpful fallback
- Shader errors: display for debugging

**User Feedback:**
- Loading progress bar for large files
- Status updates: "Parsing file... 45%"
- Tooltips on UI elements
- First-run welcome message
- Toast notifications (egui) for errors

**Error Propagation:**
- `anyhow::Result` for error handling
- Display in UI, log to console

## Dependencies

```toml
[dependencies]
# Graphics
wgpu = "0.19"
winit = "0.29"
bytemuck = "1.14"

# UI
egui = "0.27"
egui-wgpu = "0.27"
egui-winit = "0.27"

# Math
glam = "0.27"

# Terrain Generation
noise = "0.8"

# CLI & Config
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
toml = "0.8"

# Error Handling
anyhow = "1.0"
thiserror = "1.0"

# Utilities
rayon = "1.8"
```

## Implementation Roadmap

### Phase 1: Foundation (MVP)
- Basic window with wgpu setup
- Load simple .fdf file
- Parse height data into mesh
- Render as wireframe (fixed view)
- ESC to quit
- **Goal:** Terrain wireframe on screen

### Phase 2: Camera & Interaction
- Orbital camera (perspective)
- Mouse controls: rotate, zoom, pan
- Basic egui panel with camera info
- Reset button
- **Goal:** Navigate terrain smoothly

### Phase 3: Rendering Modes
- Solid shaded rendering
- Directional lighting
- Flat vs smooth normals
- Toggle wireframe/solid in UI
- **Goal:** Beautiful shaded terrain

### Phase 4: Color & Projections
- Terrain color scheme (height gradient)
- Heatmap color scheme
- Orthographic projection + toggle
- "I" key → isometric preset
- **Goal:** Multiple visual styles

### Phase 5: Advanced Rendering
- Wireframe+Solid overlay
- Contour lines
- Custom gradient editor
- Adjustable lighting UI
- Height scale slider
- **Goal:** Full rendering flexibility

### Phase 6: Procedural Generation
- Perlin noise generator
- Generator UI dialog
- CLI flag: `--generate`
- **Goal:** Built-in test terrains

### Phase 7: Performance & Polish
- Chunking system
- Frustum culling
- Adaptive strategy
- Loading progress bar
- Performance metrics
- **Goal:** Handle 8000x8000 smoothly

### Phase 8: Final Polish
- Config file support
- Camera presets
- Keyboard shortcuts
- Better error messages
- Example .fdf files
- **Goal:** Production-ready

## CLI Interface

**Basic Usage:**
```bash
lrle terrain.fdf                    # Load file with defaults
lrle --generate 512x512             # Generate procedural terrain
lrle --theme heatmap terrain.fdf    # Start with specific color scheme
lrle --projection ortho terrain.fdf # Start in orthographic mode
```

**Flags:**
- `--generate <WxH>` - Generate procedural terrain
- `--theme <name>` - Initial color scheme (terrain/heatmap/mono)
- `--projection <type>` - Initial projection (perspective/ortho)
- `--height-scale <f32>` - Initial height multiplier
- `--seed <u64>` - Random seed for generation
- `--max-size <n>` - Maximum terrain dimension before warning

All settings adjustable at runtime via UI.

## Future Enhancements (Post-MVP)

- Export rendered images/videos
- Animation/fly-through recording
- Additional noise algorithms (Worley, fractals)
- Ambient occlusion
- Shadow mapping
- Texture mapping support
- Multiple terrain comparison view
- VR support (stretch goal)

## Success Criteria

1. Load and render .fdf files smoothly
2. Handle terrains up to 8000x8000 with good performance (>30 FPS)
3. Smooth camera controls (Blender-like feel)
4. Beautiful rendering with multiple styles
5. Intuitive UI for all controls
6. Fast iteration (change settings in real-time)
7. Clean, documented Rust code
8. Cross-platform (macOS, Linux, Windows)

## Learning Goals

- Modern graphics programming (wgpu, shaders)
- 3D mathematics (transformations, projections)
- Performance optimization (chunking, culling)
- Rust + GPU programming patterns
- UI/UX design for creative tools
- Procedural generation algorithms
