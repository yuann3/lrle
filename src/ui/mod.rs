//! User interface using egui.
//!
//! Provides camera info panel and controls overlay.

use egui::Context;

use crate::renderer::camera::Camera;

/// UI state and rendering.
pub struct Ui {
    /// Whether the side panel is visible
    pub panel_visible: bool,
}

impl Ui {
    pub fn new() -> Self {
        Self {
            panel_visible: true,
        }
    }

    /// Render the UI and return whether camera was reset.
    pub fn render(&mut self, ctx: &Context, camera: &mut Camera, fps: f32) -> UiResponse {
        let mut response = UiResponse::default();

        // Toggle panel with Tab key
        if ctx.input(|i| i.key_pressed(egui::Key::Tab)) {
            self.panel_visible = !self.panel_visible;
        }

        if self.panel_visible {
            egui::SidePanel::left("controls")
                .default_width(200.0)
                .show(ctx, |ui| {
                    ui.heading("lrle");
                    ui.separator();

                    // Performance
                    ui.label(format!("FPS: {:.1}", fps));
                    ui.separator();

                    // Camera section
                    ui.collapsing("Camera", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Distance:");
                            ui.add(
                                egui::DragValue::new(&mut camera.distance)
                                    .speed(1.0)
                                    .range(1.0..=500.0),
                            );
                        });

                        ui.horizontal(|ui| {
                            ui.label("Azimuth:");
                            let mut degrees = camera.azimuth.to_degrees();
                            if ui
                                .add(egui::DragValue::new(&mut degrees).speed(1.0).suffix("°"))
                                .changed()
                            {
                                camera.azimuth = degrees.to_radians();
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Elevation:");
                            let mut degrees = camera.elevation.to_degrees();
                            if ui
                                .add(
                                    egui::DragValue::new(&mut degrees)
                                        .speed(1.0)
                                        .suffix("°")
                                        .range(-89.0..=89.0),
                                )
                                .changed()
                            {
                                camera.elevation = degrees.to_radians();
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("FOV:");
                            ui.add(
                                egui::DragValue::new(&mut camera.fov)
                                    .speed(1.0)
                                    .suffix("°")
                                    .range(10.0..=120.0),
                            );
                        });

                        if ui.button("Reset Camera").clicked() {
                            response.reset_camera = true;
                        }
                    });

                    ui.separator();

                    // Help section
                    ui.collapsing("Controls", |ui| {
                        ui.label("Left Drag: Rotate");
                        ui.label("Scroll: Zoom");
                        ui.label("Shift+Drag: Pan");
                        ui.label("Middle Drag: Pan");
                        ui.label("R: Reset Camera");
                        ui.label("Tab: Toggle Panel");
                        ui.label("ESC: Quit");
                    });
                });
        }

        response
    }
}

impl Default for Ui {
    fn default() -> Self {
        Self::new()
    }
}

/// Response from UI indicating what actions to take.
#[derive(Default)]
pub struct UiResponse {
    pub reset_camera: bool,
}
