use bytemuck::{Pod, Zeroable};

use super::TerrainData;

/// Vertex data for GPU
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

/// Generated mesh ready for GPU upload
pub struct TerrainMesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl TerrainMesh {
    /// Generate mesh from terrain data
    /// height_scale: multiplier for height values
    pub fn from_terrain(terrain: &TerrainData, height_scale: f32) -> Self {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        if terrain.width == 0 || terrain.height == 0 {
            return Self { vertices, indices };
        }

        let (min_h, max_h) = terrain.height_bounds();
        let height_range = if (max_h - min_h).abs() < 0.0001 {
            1.0
        } else {
            max_h - min_h
        };

        // Center the mesh at origin
        let offset_x = (terrain.width - 1) as f32 / 2.0;
        let offset_z = (terrain.height - 1) as f32 / 2.0;

        // Generate vertices
        for z in 0..terrain.height {
            for x in 0..terrain.width {
                let h = terrain.points[z][x];
                let y = h * height_scale;

                // Position centered at origin
                let pos = [x as f32 - offset_x, y, z as f32 - offset_z];

                // Color based on height (gradient: blue -> green -> white)
                let t = (h - min_h) / height_range;
                let color = height_to_color(t);

                vertices.push(Vertex {
                    position: pos,
                    color,
                });
            }
        }

        // Generate indices for line rendering (wireframe)
        // Horizontal lines
        for z in 0..terrain.height {
            for x in 0..terrain.width - 1 {
                let i = (z * terrain.width + x) as u32;
                indices.push(i);
                indices.push(i + 1);
            }
        }

        // Vertical lines
        for z in 0..terrain.height - 1 {
            for x in 0..terrain.width {
                let i = (z * terrain.width + x) as u32;
                indices.push(i);
                indices.push(i + terrain.width as u32);
            }
        }

        Self { vertices, indices }
    }
}

/// Convert normalized height (0-1) to RGB color
fn height_to_color(t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);

    if t < 0.3 {
        // Blue to cyan (water/low)
        let s = t / 0.3;
        [0.0, s * 0.5, 0.8 + s * 0.2]
    } else if t < 0.5 {
        // Cyan to green
        let s = (t - 0.3) / 0.2;
        [s * 0.2, 0.5 + s * 0.3, 1.0 - s * 0.6]
    } else if t < 0.8 {
        // Green to brown
        let s = (t - 0.5) / 0.3;
        [0.2 + s * 0.4, 0.8 - s * 0.4, 0.4 - s * 0.3]
    } else {
        // Brown to white (snow)
        let s = (t - 0.8) / 0.2;
        [0.6 + s * 0.4, 0.4 + s * 0.6, 0.1 + s * 0.9]
    }
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

        // Wireframe: 2 horizontal lines (2 edges) + 2 vertical lines (2 edges) = 4 edges = 8 indices
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

        // With 2x2 grid, offset is 1.0 in each direction
        // Corner positions should be centered around origin
        let positions: Vec<[f32; 3]> = mesh.vertices.iter().map(|v| v.position).collect();

        // Check that mesh spans from -1 to +1 in x and z
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
}
