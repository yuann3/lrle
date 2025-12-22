//! Input handling for camera control.
//!
//! Processes mouse and keyboard events to update camera state.

use glam::Vec3;
use winit::event::{ElementState, MouseButton, MouseScrollDelta};
use winit::keyboard::KeyCode;

use crate::renderer::camera::Camera;

/// Sensitivity constants for input handling.
pub struct InputConfig {
    /// Mouse rotation sensitivity (radians per pixel)
    pub rotate_sensitivity: f32,
    /// Mouse pan sensitivity (units per pixel)
    pub pan_sensitivity: f32,
    /// Scroll zoom sensitivity (multiplier per scroll unit)
    pub zoom_sensitivity: f32,
    /// Minimum camera distance
    pub min_distance: f32,
    /// Maximum camera distance
    pub max_distance: f32,
    /// Minimum elevation angle (radians, avoid looking straight down)
    pub min_elevation: f32,
    /// Maximum elevation angle (radians, avoid looking straight up)
    pub max_elevation: f32,
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            rotate_sensitivity: 0.005,
            pan_sensitivity: 0.1,
            zoom_sensitivity: 0.1,
            min_distance: 1.0,
            max_distance: 500.0,
            min_elevation: -std::f32::consts::FRAC_PI_2 + 0.1,
            max_elevation: std::f32::consts::FRAC_PI_2 - 0.1,
        }
    }
}

/// Tracks mouse state for drag operations.
#[derive(Default)]
pub struct InputState {
    /// Left mouse button held
    pub left_pressed: bool,
    /// Middle mouse button held
    pub middle_pressed: bool,
    /// Right mouse button held
    pub right_pressed: bool,
    /// Shift key held
    pub shift_pressed: bool,
    /// Last mouse position (for computing delta)
    pub last_mouse_pos: Option<(f32, f32)>,
}

impl InputState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if we should be rotating (left drag without shift)
    pub fn is_rotating(&self) -> bool {
        self.left_pressed && !self.shift_pressed
    }

    /// Check if we should be panning (middle drag OR shift+left drag)
    pub fn is_panning(&self) -> bool {
        self.middle_pressed || (self.left_pressed && self.shift_pressed)
    }
}

/// Input controller that processes events and updates camera.
pub struct InputController {
    pub config: InputConfig,
    pub state: InputState,
}

impl InputController {
    pub fn new() -> Self {
        Self {
            config: InputConfig::default(),
            state: InputState::new(),
        }
    }

    /// Handle mouse button press/release.
    pub fn handle_mouse_button(&mut self, button: MouseButton, state: ElementState) {
        let pressed = state == ElementState::Pressed;
        match button {
            MouseButton::Left => self.state.left_pressed = pressed,
            MouseButton::Middle => self.state.middle_pressed = pressed,
            MouseButton::Right => self.state.right_pressed = pressed,
            _ => {}
        }
    }

    /// Handle keyboard key press/release.
    pub fn handle_keyboard(&mut self, key: KeyCode, state: ElementState, camera: &mut Camera) {
        let pressed = state == ElementState::Pressed;

        match key {
            KeyCode::ShiftLeft | KeyCode::ShiftRight => {
                self.state.shift_pressed = pressed;
            }
            KeyCode::KeyR if pressed => {
                // Reset camera to default
                *camera = Camera::new();
            }
            _ => {}
        }
    }

    /// Handle mouse movement. Returns true if camera was updated.
    pub fn handle_mouse_move(&mut self, x: f32, y: f32, camera: &mut Camera) -> bool {
        let mut updated = false;

        if let Some((last_x, last_y)) = self.state.last_mouse_pos {
            let dx = x - last_x;
            let dy = y - last_y;

            if self.state.is_rotating() {
                self.rotate_camera(camera, dx, dy);
                updated = true;
            } else if self.state.is_panning() {
                self.pan_camera(camera, dx, dy);
                updated = true;
            }
        }

        self.state.last_mouse_pos = Some((x, y));
        updated
    }

    /// Handle mouse scroll for zooming.
    pub fn handle_scroll(&mut self, delta: MouseScrollDelta, camera: &mut Camera) {
        let scroll_amount = match delta {
            MouseScrollDelta::LineDelta(_, y) => y,
            MouseScrollDelta::PixelDelta(pos) => pos.y as f32 * 0.1,
        };

        self.zoom_camera(camera, scroll_amount);
    }

    /// Rotate camera based on mouse delta.
    fn rotate_camera(&self, camera: &mut Camera, dx: f32, dy: f32) {
        // Horizontal movement rotates azimuth
        camera.azimuth -= dx * self.config.rotate_sensitivity;

        // Vertical movement changes elevation
        camera.elevation += dy * self.config.rotate_sensitivity;

        // Clamp elevation to avoid gimbal lock
        camera.elevation = camera
            .elevation
            .clamp(self.config.min_elevation, self.config.max_elevation);
    }

    /// Pan camera target based on mouse delta.
    fn pan_camera(&self, camera: &mut Camera, dx: f32, dy: f32) {
        // Calculate camera right and up vectors for panning
        let forward = (camera.target - camera.position()).normalize();
        let right = forward.cross(Vec3::Y).normalize();
        let up = right.cross(forward).normalize();

        // Scale pan by distance (feels more natural)
        let scale = camera.distance * self.config.pan_sensitivity * 0.01;

        // Move target in screen space
        camera.target -= right * dx * scale;
        camera.target += up * dy * scale;
    }

    /// Zoom camera by adjusting distance.
    fn zoom_camera(&self, camera: &mut Camera, scroll: f32) {
        // Exponential zoom feels more natural
        let factor = 1.0 - scroll * self.config.zoom_sensitivity;
        camera.distance *= factor;

        // Clamp distance
        camera.distance = camera
            .distance
            .clamp(self.config.min_distance, self.config.max_distance);
    }
}

impl Default for InputController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_state_default() {
        let state = InputState::new();
        assert!(!state.left_pressed);
        assert!(!state.is_rotating());
        assert!(!state.is_panning());
    }

    #[test]
    fn test_rotation_detection() {
        let mut state = InputState::new();
        state.left_pressed = true;
        assert!(state.is_rotating());
        assert!(!state.is_panning());

        state.shift_pressed = true;
        assert!(!state.is_rotating());
        assert!(state.is_panning());
    }

    #[test]
    fn test_pan_detection() {
        let mut state = InputState::new();

        // Middle button pans
        state.middle_pressed = true;
        assert!(state.is_panning());

        // Shift+Left also pans
        state.middle_pressed = false;
        state.left_pressed = true;
        state.shift_pressed = true;
        assert!(state.is_panning());
    }

    #[test]
    fn test_mouse_button_handling() {
        let mut controller = InputController::new();

        controller.handle_mouse_button(MouseButton::Left, ElementState::Pressed);
        assert!(controller.state.left_pressed);

        controller.handle_mouse_button(MouseButton::Left, ElementState::Released);
        assert!(!controller.state.left_pressed);
    }

    #[test]
    fn test_camera_reset() {
        let mut controller = InputController::new();
        let mut camera = Camera::new();

        // Modify camera
        camera.distance = 100.0;
        camera.azimuth = 1.5;

        // Press R to reset
        controller.handle_keyboard(KeyCode::KeyR, ElementState::Pressed, &mut camera);

        // Camera should be reset to defaults
        assert_eq!(camera.distance, 50.0);
    }

    #[test]
    fn test_zoom_limits() {
        let mut controller = InputController::new();
        let mut camera = Camera::new();

        // Zoom way in
        for _ in 0..100 {
            controller.handle_scroll(MouseScrollDelta::LineDelta(0.0, 1.0), &mut camera);
        }
        assert!(camera.distance >= controller.config.min_distance);

        // Zoom way out
        for _ in 0..100 {
            controller.handle_scroll(MouseScrollDelta::LineDelta(0.0, -1.0), &mut camera);
        }
        assert!(camera.distance <= controller.config.max_distance);
    }

    #[test]
    fn test_elevation_limits() {
        let mut controller = InputController::new();
        let mut camera = Camera::new();
        camera.elevation = 0.0;

        // Simulate large upward drag
        controller.state.left_pressed = true;
        controller.state.last_mouse_pos = Some((0.0, 0.0));
        controller.handle_mouse_move(0.0, 1000.0, &mut camera);

        assert!(camera.elevation <= controller.config.max_elevation);

        // Simulate large downward drag
        controller.state.last_mouse_pos = Some((0.0, 0.0));
        controller.handle_mouse_move(0.0, -2000.0, &mut camera);

        assert!(camera.elevation >= controller.config.min_elevation);
    }
}
