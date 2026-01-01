//! User interface using egui.
//!
//! Provides camera info panel, render mode selection, and lighting controls.

use egui::Context;

use crate::renderer::camera::Camera;
use crate::renderer::{LightingConfig, RenderMode};
use crate::renderer::Projection;
use crate::terrain::ColorScheme;

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

    /// Render the UI and return response actions.
    pub fn render(
        &mut self,
        ctx: &Context,
        camera: &mut Camera,
        render_mode: &mut RenderMode,
        color_scheme: &mut ColorScheme,
        lighting: &mut LightingConfig,
        fps: f32,
    ) -> UiResponse {
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

                    // Rendering section
                    ui.collapsing("Rendering", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Mode:");
                            egui::ComboBox::from_id_salt("render_mode")
                                .selected_text(match render_mode {
                                    RenderMode::Wireframe => "Wireframe",
                                    RenderMode::Solid => "Solid",
                                    RenderMode::Both => "Both",
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        render_mode,
                                        RenderMode::Wireframe,
                                        "Wireframe",
                                    );
                                    ui.selectable_value(render_mode, RenderMode::Solid, "Solid");
                                    ui.selectable_value(render_mode, RenderMode::Both, "Both");
                                });
                        });

                        ui.horizontal(|ui| {
                            ui.label("Colors:");
                            egui::ComboBox::from_id_salt("color_scheme")
                                .selected_text(match color_scheme {
                                    ColorScheme::Terrain => "Terrain",
                                    ColorScheme::Heatmap => "Heatmap",
                                    ColorScheme::Monochrome => "Monochrome",
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        color_scheme,
                                        ColorScheme::Terrain,
                                        "Terrain",
                                    );
                                    ui.selectable_value(
                                        color_scheme,
                                        ColorScheme::Heatmap,
                                        "Heatmap",
                                    );
                                    ui.selectable_value(
                                        color_scheme,
                                        ColorScheme::Monochrome,
                                        "Monochrome",
                                    );
                                });
                        });
                    });

                    ui.separator();

                    // Lighting section (only shown for solid/both modes)
                    if matches!(render_mode, RenderMode::Solid | RenderMode::Both) {
                        ui.collapsing("Lighting", |ui| {
                            // Light direction as azimuth/elevation
                            let mut light_azimuth = lighting
                                .direction
                                .z
                                .atan2(lighting.direction.x)
                                .to_degrees();
                            let mut light_elevation = lighting.direction.y.asin().to_degrees();

                            ui.horizontal(|ui| {
                                ui.label("Azimuth:");
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut light_azimuth)
                                            .speed(1.0)
                                            .suffix("°"),
                                    )
                                    .changed()
                                {
                                    update_light_direction(
                                        lighting,
                                        light_azimuth,
                                        light_elevation,
                                    );
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.label("Elevation:");
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut light_elevation)
                                            .speed(1.0)
                                            .suffix("°")
                                            .range(-90.0..=90.0),
                                    )
                                    .changed()
                                {
                                    update_light_direction(
                                        lighting,
                                        light_azimuth,
                                        light_elevation,
                                    );
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.label("Ambient:");
                                ui.add(
                                    egui::Slider::new(&mut lighting.ambient, 0.0..=1.0)
                                        .show_value(true),
                                );
                            });

                            if ui.button("Reset Lighting").clicked() {
                                *lighting = LightingConfig::default();
                            }
                        });

                        ui.separator();
                    }

                    // Camera section
                    ui.collapsing("Camera", |ui| {
                       ui.horizontal(|ui| {
                           ui.label("Projection:");
                           egui::ComboBox::from_id_salt("projection")
                               .selected_text(match camera.projection {
                                   Projection::Perspective => "Perspective",
                                   Projection::Orthographic => "Orthographic",
                               })
                               .show_ui(ui, |ui| {
                                   ui.selectable_value(
                                       &mut camera.projection,
                                       Projection::Perspective,
                                       "Perspective",
                                   );
                                   ui.selectable_value(
                                       &mut camera.projection,
                                       Projection::Orthographic,
                                       "Orthographic",
                                   );
                               });
                       });

                       if ui.button("Isometric View").clicked() {
                           camera.set_isometric();
                       }

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
                        ui.label("P: Toggle Projection");
                        ui.label("I: Isometric View");
                        ui.label("R: Reset Camera");
                        ui.label("Tab: Toggle Panel");
                        ui.label("ESC: Quit");
                    });
                });
        }

        response
    }
}

fn update_light_direction(lighting: &mut LightingConfig, azimuth_deg: f32, elevation_deg: f32) {
    let azimuth = azimuth_deg.to_radians();
    let elevation = elevation_deg.to_radians();
    lighting.direction = glam::Vec3::new(
        elevation.cos() * azimuth.cos(),
        elevation.sin(),
        elevation.cos() * azimuth.sin(),
    )
    .normalize();
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
