//! Terrain mesh generation for GPU rendering.
//!
//! Converts [`TerrainData`] into GPU-ready vertex and index buffers
//! for wireframe rendering.

use bytemuck::{Pod, Zeroable};
use glam::Vec3;

use super::colors::{height_to_color, ColorScheme};
use super::TerrainData;

/// GPU vertex data with position and color.
///
/// Uses `repr(C)` for consistent memory layout matching the shader.
/// Derives `Pod` and `Zeroable` for safe GPU buffer operations.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    /// 3D position (x, y, z)
    pub position: [f32; 3],
    /// RGB color (normalized 0.0-1.0)
    pub color: [f32; 3],
    /// Surface normal
    pub normal: [f32; 3],
}

impl Vertex {
    /// Returns the vertex buffer layout descriptor for wgpu.
    ///
    /// Layout:
    /// - Location 0: position (vec3<f32>)
    /// - Location 1: color (vec3<f32>)
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // Color
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // Normal
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 3]>() * 2) as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

/// Shading mode for normal calculation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShadingMode {
    /// Flat shading - normals from height gradient
    Flat,
    /// Smooth shading - normals averaged at vertices
    #[default]
    Smooth,
}

/// Generated mesh ready for GPU upload.
///
/// Contains vertices and indices for wireframe line rendering.
/// The mesh is centered at the origin for orbital camera rotation.
pub struct TerrainMesh {
    /// Vertex data (position + color per vertex)
    pub vertices: Vec<Vertex>,
    /// Index pairs for line segments (LineList topology)
    pub indices: Vec<u32>,
    /// Triangle indices for solid rendering (TriangleList)
    pub triangle_indices: Vec<u32>,
}

impl TerrainMesh {
    /// Generate mesh with default smooth shading and terrain color scheme.
    pub fn from_terrain(terrain: &TerrainData, height_scale: f32) -> Self {
        Self::from_terrain_with_options(terrain, height_scale, ShadingMode::Smooth, ColorScheme::Terrain)
    }

    /// Generate mesh with specified shading mode and default terrain color scheme.
    pub fn from_terrain_with_shading(
        terrain: &TerrainData,
        height_scale: f32,
        shading_mode: ShadingMode,
    ) -> Self {
        Self::from_terrain_with_options(terrain, height_scale, shading_mode, ColorScheme::Terrain)
    }

    /// Generate mesh from terrain data with all options.
    ///
    /// # Arguments
    ///
    /// * `terrain` - Source terrain height data
    /// * `height_scale` - Multiplier for height values (Y axis)
    /// * `shading_mode` - Flat or smooth shading for normals
    /// * `color_scheme` - Color gradient scheme for height coloring
    ///
    /// # Returns
    ///
    /// A mesh with:
    /// - Vertices positioned in 3D space, centered at origin
    /// - Height-based gradient coloring using the specified scheme
    /// - Surface normals for lighting
    /// - Index pairs for horizontal and vertical wireframe lines
    /// - Triangle indices for solid rendering
    pub fn from_terrain_with_options(
        terrain: &TerrainData,
        height_scale: f32,
        shading_mode: ShadingMode,
        color_scheme: ColorScheme,
    ) -> Self {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        if terrain.width == 0 || terrain.height == 0 {
            return Self {
                vertices,
                indices,
                triangle_indices: Vec::new(),
            };
        }

        let (min_h, max_h) = terrain.height_bounds();
        let height_range = if (max_h - min_h).abs() < f32::EPSILON {
            1.0
        } else {
            max_h - min_h
        };

        // Center the mesh at origin for orbital camera
        let offset_x = (terrain.width - 1) as f32 / 2.0;
        let offset_z = (terrain.height - 1) as f32 / 2.0;

        // first, generate positions and colors
        let mut positions = Vec::with_capacity(terrain.width * terrain.height);
        let mut colors = Vec::with_capacity(terrain.width * terrain.height);

        for z in 0..terrain.height {
            for x in 0..terrain.width {
                let h = terrain.points[z][x];
                let y = h * height_scale;

                positions.push(Vec3::new(x as f32 - offset_x, y, z as f32 - offset_z));

                let t = (h - min_h) / height_range;
                colors.push(height_to_color(t, color_scheme));
            }
        }

        let normals = match shading_mode {
            ShadingMode::Smooth => calculate_smooth_normals(terrain, &positions),
            ShadingMode::Flat => calculate_flat_normals(terrain, &positions),
        };

        for i in 0..positions.len() {
            vertices.push(Vertex {
                position: positions[i].to_array(),
                color: colors[i],
                normal: normals[i].to_array(),
            });
        }

        // Generate indices for wireframe (LineList topology)
        // Horizontal lines (along X axis)
        for z in 0..terrain.height {
            for x in 0..terrain.width - 1 {
                let i = (z * terrain.width + x) as u32;
                indices.push(i);
                indices.push(i + 1);
            }
        }

        // Vertical lines (along Z axis)
        for z in 0..terrain.height - 1 {
            for x in 0..terrain.width {
                let i = (z * terrain.width + x) as u32;
                indices.push(i);
                indices.push(i + terrain.width as u32);
            }
        }

        let mut triangle_indices = Vec::new();
        for z in 0..terrain.height - 1 {
            for x in 0..terrain.width - 1 {
                let top_left = (z * terrain.width + x) as u32;
                let top_right = top_left + 1;
                let bottom_left = top_left + terrain.width as u32;
                let bottom_right = bottom_left + 1;

                triangle_indices.push(top_left);
                triangle_indices.push(bottom_left);
                triangle_indices.push(top_right);

                triangle_indices.push(top_right);
                triangle_indices.push(bottom_left);
                triangle_indices.push(bottom_right);
            }
        }

        Self {
            vertices,
            indices,
            triangle_indices,
        }
    }
}

/// Calculate smooth normals by averaging face normals at each vertex
fn calculate_smooth_normals(terrain: &TerrainData, positions: &[Vec3]) -> Vec<Vec3> {
    let width = terrain.width;
    let height = terrain.height;
    let mut normals = vec![Vec3::ZERO; positions.len()];

    for z in 0..height - 1 {
        for x in 0..width - 1 {
            let idx = z * width + x;
            let tl = positions[idx];
            let tr = positions[idx + 1];
            let bl = positions[idx + width];
            let br = positions[idx + width + 1];

            // first triangle normal
            let n1 = (bl - tl).cross(tr - tl).normalize_or_zero();
            // second triangle normal
            let n2 = (bl - tr).cross(br - tr).normalize_or_zero();

            // add to all vertices of each triange
            normals[idx] += n1;
            normals[idx + width] += n1 + n2;
            normals[idx + 1] += n1 + n2;
            normals[idx + width + 1] += n2;
        }
    }

    for n in &mut normals {
        *n = n.normalize_or_zero();
        if n.y < 0.0 {
            *n = -*n;
        }
    }

    normals
}

/// Calculate flat normals from height gradient at each vertex
fn calculate_flat_normals(terrain: &TerrainData, positions: &[Vec3]) -> Vec<Vec3> {
    let width = terrain.width;
    let height = terrain.height;
    let mut normals = vec![Vec3::Y; positions.len()];

    for z in 0..height {
        for x in 0..width {
            let idx = z * width + x;

            let dx = if x == 0 {
                positions[idx + 1].y - positions[idx].y
            } else if x == width - 1 {
                positions[idx].y - positions[idx - 1].y
            } else {
                (positions[idx + 1].y - positions[idx - 1].y) / 2.0
            };

            let dz = if z == 0 {
                positions[idx + width].y - positions[idx].y
            } else if z == height - 1 {
                positions[idx].y - positions[idx - width].y
            } else {
                (positions[idx + width].y - positions[idx - width].y) / 2.0
            };

            normals[idx] = Vec3::new(-dx, 1.0, -dz).normalize_or_zero();
        }
    }

    normals
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesh_from_simple_terrain() {
        let terrain = TerrainData::new(vec![vec![0.0, 1.0], vec![2.0, 3.0]], None);
        let mesh = TerrainMesh::from_terrain(&terrain, 1.0);

        // 2x2 grid = 4 vertices
        assert_eq!(mesh.vertices.len(), 4);

        // Wireframe: 2 horizontal + 2 vertical edges = 4 edges = 8 indices
        assert_eq!(mesh.indices.len(), 8);
    }

    #[test]
    fn test_mesh_empty_terrain() {
        let terrain = TerrainData::new(vec![], None);
        let mesh = TerrainMesh::from_terrain(&terrain, 1.0);

        assert!(mesh.vertices.is_empty());
        assert!(mesh.indices.is_empty());
    }

    #[test]
    fn test_mesh_centered_at_origin() {
        let terrain = TerrainData::new(vec![vec![0.0, 0.0], vec![0.0, 0.0]], None);
        let mesh = TerrainMesh::from_terrain(&terrain, 1.0);

        let positions: Vec<[f32; 3]> = mesh.vertices.iter().map(|v| v.position).collect();

        // With 2x2 grid and (width-1)/2 offset, mesh spans from -0.5 to +0.5
        let min_x = positions.iter().map(|p| p[0]).fold(f32::MAX, f32::min);
        let max_x = positions.iter().map(|p| p[0]).fold(f32::MIN, f32::max);

        assert!(min_x < 0.0);
        assert!(max_x > 0.0);
    }

    #[test]
    fn test_height_scale() {
        let terrain = TerrainData::new(vec![vec![10.0]], None);

        let mesh1 = TerrainMesh::from_terrain(&terrain, 1.0);
        let mesh2 = TerrainMesh::from_terrain(&terrain, 2.0);

        assert_eq!(mesh1.vertices[0].position[1], 10.0);
        assert_eq!(mesh2.vertices[0].position[1], 20.0);
    }

    #[test]
    fn test_height_to_color_bounds() {
        // Test terrain color gradient at key points
        let low = height_to_color(0.0, ColorScheme::Terrain);
        let mid = height_to_color(0.5, ColorScheme::Terrain);
        let high = height_to_color(1.0, ColorScheme::Terrain);

        // Low should be bluish
        assert!(low[2] > low[0]);
        // Mid should be greenish
        assert!(mid[1] > mid[0]);
        // High should be whitish
        assert!(high[0] > 0.9 && high[1] > 0.9 && high[2] > 0.9);
    }
}
