use glam::{Mat4, Vec3};

pub struct Camera {
    /// Distance from target
    pub distance: f32,
    /// Horizontal rotation (radians)
    pub azimuth: f32,
    /// Vertical rotation (radians), clamped
    pub elevation: f32,
    /// Look-at target point
    pub target: Vec3,
    /// Field of view in degrees
    pub fov: f32,
    /// Near clipping plane
    pub near: f32,
    /// Far clipping plane
    pub far: f32,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            distance: 50.0,
            azimuth: std::f32::consts::FRAC_PI_4,   // 45 degrees
            elevation: std::f32::consts::FRAC_PI_6, // 30 degrees
            target: Vec3::ZERO,
            fov: 60.0,
            near: 0.1,
            far: 1000.0,
        }
    }

    /// Calculate camera position from orbital parameters
    pub fn position(&self) -> Vec3 {
        let x = self.distance * self.elevation.cos() * self.azimuth.sin();
        let y = self.distance * self.elevation.sin();
        let z = self.distance * self.elevation.cos() * self.azimuth.cos();
        self.target + Vec3::new(x, y, z)
    }

    /// Build view matrix (camera transform)
    pub fn build_view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position(), self.target, Vec3::Y)
    }

    /// Build perspective projection matrix
    pub fn build_projection_matrix(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh(self.fov.to_radians(), aspect, self.near, self.far)
    }

    /// Combined view-projection matrix
    pub fn build_view_projection_matrix(&self, aspect: f32) -> Mat4 {
        self.build_projection_matrix(aspect) * self.build_view_matrix()
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_default() {
        let camera = Camera::new();
        assert_eq!(camera.distance, 50.0);
        assert_eq!(camera.target, Vec3::ZERO);
    }

    #[test]
    fn test_camera_position() {
        let mut camera = Camera::new();
        camera.distance = 10.0;
        camera.azimuth = 0.0;
        camera.elevation = 0.0;

        let pos = camera.position();
        // At azimuth=0, elevation=0, camera should be on +Z axis
        assert!((pos.x).abs() < 0.001);
        assert!((pos.y).abs() < 0.001);
        assert!((pos.z - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_view_projection_matrix() {
        let camera = Camera::new();
        let vp = camera.build_view_projection_matrix(1.0);
        // Just verify it produces a valid matrix (non-zero)
        assert!(vp.determinant().abs() > 0.0001);
    }
}
