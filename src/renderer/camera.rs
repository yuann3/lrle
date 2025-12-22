//! Orbital camera for 3D terrain viewing.
//!
//! Provides an orbital (arcball-style) camera that rotates around a target point.
//! Supports perspective projection with configurable field of view.

use glam::{Mat4, Vec3};

/// Orbital camera that rotates around a target point.
///
/// Uses spherical coordinates (distance, azimuth, elevation) to position
/// the camera relative to a target. Supports perspective projection.
///
/// # Coordinate System
///
/// - Azimuth: Horizontal rotation around Y axis (0 = +Z direction)
/// - Elevation: Vertical angle from XZ plane (clamped to avoid gimbal lock)
/// - Distance: Distance from target point
pub struct Camera {
    /// Distance from target point
    pub distance: f32,

    /// Horizontal rotation in radians (0 = looking along +Z)
    pub azimuth: f32,

    /// Vertical rotation in radians (0 = horizontal, positive = looking down)
    pub elevation: f32,

    /// Point the camera looks at (center of rotation)
    pub target: Vec3,

    /// Vertical field of view in degrees
    pub fov: f32,

    /// Near clipping plane distance
    pub near: f32,

    /// Far clipping plane distance
    pub far: f32,
}

impl Camera {
    /// Create a new camera with default settings.
    ///
    /// Default position is at 45° azimuth and 30° elevation,
    /// looking at the origin from a distance of 50 units.
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

    /// Calculate camera position in world space from orbital parameters.
    ///
    /// Converts spherical coordinates (distance, azimuth, elevation) to
    /// Cartesian coordinates relative to the target point.
    pub fn position(&self) -> Vec3 {
        let x = self.distance * self.elevation.cos() * self.azimuth.sin();
        let y = self.distance * self.elevation.sin();
        let z = self.distance * self.elevation.cos() * self.azimuth.cos();
        self.target + Vec3::new(x, y, z)
    }

    /// Build the view matrix (world to camera transform).
    ///
    /// Uses right-handed look-at with Y-up convention.
    pub fn build_view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position(), self.target, Vec3::Y)
    }

    /// Build the perspective projection matrix.
    ///
    /// # Arguments
    ///
    /// * `aspect` - Width/height aspect ratio of the viewport
    pub fn build_projection_matrix(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh(self.fov.to_radians(), aspect, self.near, self.far)
    }

    /// Build combined view-projection matrix.
    ///
    /// This is the matrix sent to shaders for transforming vertices
    /// from world space to clip space.
    ///
    /// # Arguments
    ///
    /// * `aspect` - Width/height aspect ratio of the viewport
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
        assert_eq!(camera.fov, 60.0);
    }

    #[test]
    fn test_camera_position_at_zero_angles() {
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
    fn test_camera_position_at_90_azimuth() {
        let mut camera = Camera::new();
        camera.distance = 10.0;
        camera.azimuth = std::f32::consts::FRAC_PI_2; // 90 degrees
        camera.elevation = 0.0;

        let pos = camera.position();

        // At azimuth=90°, camera should be on +X axis
        assert!((pos.x - 10.0).abs() < 0.001);
        assert!((pos.y).abs() < 0.001);
        assert!((pos.z).abs() < 0.001);
    }

    #[test]
    fn test_view_projection_matrix_valid() {
        let camera = Camera::new();
        let vp = camera.build_view_projection_matrix(1.0);

        // Matrix should be non-singular (valid transform)
        assert!(vp.determinant().abs() > 0.0001);
    }

    #[test]
    fn test_camera_with_offset_target() {
        let mut camera = Camera::new();
        camera.distance = 10.0;
        camera.azimuth = 0.0;
        camera.elevation = 0.0;
        camera.target = Vec3::new(5.0, 0.0, 0.0);

        let pos = camera.position();

        // Camera should be offset by target position
        assert!((pos.x - 5.0).abs() < 0.001);
        assert!((pos.z - 10.0).abs() < 0.001);
    }
}
