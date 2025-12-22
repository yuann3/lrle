pub mod loader;
pub mod mesh;

pub use loader::load_fdf;
pub use mesh::TerrainMesh;
pub use mesh::Vertex;

#[derive(Debug, Clone)]
pub struct TerrainData {
    pub width: usize,
    pub height: usize,
    pub points: Vec<Vec<f32>>,
    pub colors: Option<Vec<Vec<u32>>>,
}

impl TerrainData {
    pub fn new(points: Vec<Vec<f32>>, colors: Option<Vec<Vec<u32>>>) -> Self {
        let height = points.len();
        let width = points.first().map(|r| r.len()).unwrap_or(0);
        Self {
            width,
            height,
            points,
            colors,
        }
    }

    /// Get min and max height values
    pub fn height_bounds(&self) -> (f32, f32) {
        let mut min = f32::MAX;
        let mut max = f32::MIN;
        for row in &self.points {
            for &h in row {
                min = min.min(h);
                max = max.max(h);
            }
        }
        if min > max { (0.0, 0.0) } else { (min, max) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_data_new() {
        let points = vec![vec![0.0, 1.0, 2.0], vec![3.0, 4.0, 5.0]];
        let terrain = TerrainData::new(points, None);

        assert_eq!(terrain.width, 3);
        assert_eq!(terrain.height, 2);
        assert!(terrain.colors.is_none());
    }

    #[test]
    fn test_terrain_data_empty() {
        let terrain = TerrainData::new(vec![], None);
        assert_eq!(terrain.width, 0);
        assert_eq!(terrain.height, 0);
    }

    #[test]
    fn test_height_bounds() {
        let points = vec![vec![0.0, 5.0, 2.0], vec![-3.0, 4.0, 10.0]];
        let terrain = TerrainData::new(points, None);
        let (min, max) = terrain.height_bounds();

        assert_eq!(min, -3.0);
        assert_eq!(max, 10.0);
    }

    #[test]
    fn test_height_bounds_empty() {
        let terrain = TerrainData::new(vec![], None);
        let (min, max) = terrain.height_bounds();
        assert_eq!(min, 0.0);
        assert_eq!(max, 0.0);
    }
}
