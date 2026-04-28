// ╔══════════════════════════════════════════════════════════════════════════════╗
// ║          IWES / LUH — 3D Viewport Controls                                ║
// ║                                                                           ║
// ║  Blender-style 3D camera controls for eframe/egui apps.                  ║
// ║  Provides orbit, pan, zoom, FPS mode, keyboard view presets,             ║
// ║  and simple origin gizmo (colored axis lines).                            ║
// ║                                                                           ║
// ║  Reference: 3d-viewport-controls-spec.md                                 ║
// ║  Author: Basem Rajjoub, LUH / IWES                                       ║
// ╚══════════════════════════════════════════════════════════════════════════════╝

use eframe::egui;

// ══════════════════════════════════════════════════════════════════════════════
// ── INTERACTION STATE MACHINE ──
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InteractionState {
    Idle,
    Orbiting,
    Panning,
    Zooming,
    Fps,
}

// ══════════════════════════════════════════════════════════════════════════════
// ── AXIS COLORS (from spec) ──
// ══════════════════════════════════════════════════════════════════════════════

pub const AXIS_X_COLOR: egui::Color32 = egui::Color32::from_rgb(0xFF, 0x33, 0x53); // Red
pub const AXIS_Y_COLOR: egui::Color32 = egui::Color32::from_rgb(0x33, 0xFF, 0x57); // Green
pub const AXIS_Z_COLOR: egui::Color32 = egui::Color32::from_rgb(0x33, 0x85, 0xFF); // Blue

// ══════════════════════════════════════════════════════════════════════════════
// ── CONSTANTS ──
// ══════════════════════════════════════════════════════════════════════════════

/// Orbit sensitivity: ~0.3°/pixel = 0.00524 rad/pixel
const ORBIT_SENSITIVITY: f32 = 0.00524;
/// FPS look sensitivity: 0.002 rad/pixel (per spec)
const FPS_SENSITIVITY: f32 = 0.002;
/// Elevation clamp: ±89° in radians
const ELEVATION_CLAMP: f32 = 1.553;

// ══════════════════════════════════════════════════════════════════════════════
// ── VIEWPORT3D STRUCT ──
// ══════════════════════════════════════════════════════════════════════════════

pub struct Viewport3D {
    // -- Orbital camera state --
    pub rotation_x: f32,
    pub rotation_y: f32,
    pub zoom: f32,
    /// Screen-space pan offset (pixels).
    pub pan_x: f32,
    pub pan_y: f32,
    /// 3D pivot point for orbital rotation (rotation orbits around this point).
    pub pivot: [f32; 3],

    // -- FPS camera state --
    /// FPS free-flight camera mode.
    pub fps_mode: bool,
    pub cam_x: f32,
    pub cam_y: f32,
    pub cam_z: f32,
    pub cam_yaw: f32,
    pub cam_pitch: f32,
    /// FPS movement speed (adjustable via scroll, shift = 2x).
    pub fps_speed: f32,

    // -- Projection --
    /// Perspective projection in orbital mode (false = orthographic).
    pub perspective: bool,

    // -- Origin gizmo --
    /// Show colored XYZ axis lines at origin.
    pub show_origin_gizmo: bool,

    // -- Internal --
    state: InteractionState,
}

/// Backward-compatibility alias. Existing code using `View3D` compiles unchanged.
pub type View3D = Viewport3D;

impl Default for Viewport3D {
    fn default() -> Self {
        Self {
            rotation_x: 0.3,
            rotation_y: 0.4,
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            pivot: [0.0, 0.0, 0.0],
            fps_mode: false,
            cam_x: 0.0,
            cam_y: 0.0,
            cam_z: -3.0,
            cam_yaw: 0.0,
            cam_pitch: 0.0,
            fps_speed: 0.05,
            perspective: false,
            show_origin_gizmo: true,
            state: InteractionState::Idle,
        }
    }
}

impl Viewport3D {
    // ══════════════════════════════════════════════════════════════════════════
    // ── PROJECTION ──
    // ══════════════════════════════════════════════════════════════════════════

    /// Project a 3D point (x, y, z) onto 2D screen coordinates.
    /// Supports both orbital (default) and FPS camera modes.
    pub fn project(&self, x: f32, y: f32, z: f32, center: egui::Pos2, scale: f32) -> egui::Pos2 {
        if self.fps_mode {
            let dx = x - self.cam_x;
            let dy = y - self.cam_y;
            let dz = z - self.cam_z;

            let (cy, sy) = (self.cam_yaw.cos(), self.cam_yaw.sin());
            let (cp, sp) = (self.cam_pitch.cos(), self.cam_pitch.sin());

            let rx = dx * cy + dz * sy;
            let rz = -dx * sy + dz * cy;
            let ry = dy * cp - rz * sp;
            let rz2 = dy * sp + rz * cp;

            let depth = rz2.max(0.1);
            let fov_scale = scale * self.zoom * 2.0;
            egui::Pos2::new(
                center.x + self.pan_x + rx / depth * fov_scale,
                center.y + self.pan_y - ry / depth * fov_scale,
            )
        } else {
            // Subtract pivot so rotation orbits around the pivot point
            let px = x - self.pivot[0];
            let py = y - self.pivot[1];
            let pz = z - self.pivot[2];
            let (cy, sy) = (self.rotation_y.cos(), self.rotation_y.sin());
            let (cx, sx) = (self.rotation_x.cos(), self.rotation_x.sin());
            let rx = px * cy + pz * sy;
            let rz = -px * sy + pz * cy;
            let ry = py * cx - rz * sx;
            let s = scale * self.zoom;
            if self.perspective {
                let rz2 = py * sx + rz * cx;
                let d = (rz2 + 5.0).max(0.5);
                let ps = 3.0 / d;
                egui::Pos2::new(center.x + self.pan_x + rx * s * ps, center.y + self.pan_y - ry * s * ps)
            } else {
                egui::Pos2::new(center.x + self.pan_x + rx * s, center.y + self.pan_y - ry * s)
            }
        }
    }

    // ══════════════════════════════════════════════════════════════════════════
    // ── INPUT HANDLING ──
    // ══════════════════════════════════════════════════════════════════════════

    /// Handle mouse drag (rotation/pan), scroll (zoom), and keyboard input.
    ///
    /// **Orbital mode** (Blender-style):
    /// - LMB drag / MMB drag / Alt+LMB drag = orbit
    /// - Shift+LMB / Shift+MMB / Alt+Shift+LMB = pan
    /// - Ctrl+MMB drag = smooth zoom (vertical)
    /// - Scroll = zoom toward cursor
    /// - Double-click / Home = reset view
    ///
    /// **FPS mode** (Half-Life style):
    /// - LMB drag = look (inverted Y)
    /// - WASD = move, E/Space = up, Q = down
    /// - Shift = 2x speed, Scroll = adjust base speed
    pub fn handle_input(&mut self, response: &egui::Response, ui: &egui::Ui) {
        let (shift, ctrl, alt) = ui.input(|i| (i.modifiers.shift, i.modifiers.ctrl, i.modifiers.alt));

        if self.fps_mode {
            self.state = InteractionState::Fps;

            // Mouse look: mouse-right = look right, mouse-up = look up
            // Derived from projection: rx = dx*cos(yaw) + dz*sin(yaw)
            //   increasing yaw shifts objects right → camera turns left
            //   so yaw must DECREASE for "look right" → yaw -= delta.x
            // Pitch: screen Y is inverted → pitch -= delta.y for "look up"
            if response.dragged_by(egui::PointerButton::Primary) {
                let delta = response.drag_delta();
                self.cam_yaw -= delta.x * FPS_SENSITIVITY;
                self.cam_pitch -= delta.y * FPS_SENSITIVITY;
                self.cam_pitch = self.cam_pitch.clamp(-ELEVATION_CLAMP, ELEVATION_CLAMP);
            }

            // WASD + E/Space/Q movement (proper FPS: W/S move toward where you look)
            // Front vector derived from projection inverse:
            //   forward projects to screen center (rx=0, ry=0, depth>0)
            //   → front = (-cos(pitch)*sin(yaw), sin(pitch), cos(pitch)*cos(yaw))
            // Right vector (rx=1 on screen):
            //   → right = (cos(yaw), 0, sin(yaw))
            if response.hovered() {
                let actual_speed = self.fps_speed * if shift { 2.0 } else { 1.0 };
                let (cy, sy) = (self.cam_yaw.cos(), self.cam_yaw.sin());
                let (cp, sp) = (self.cam_pitch.cos(), self.cam_pitch.sin());

                let front_x = -cp * sy;
                let front_y = sp;
                let front_z = cp * cy;
                let right_x = cy;
                let right_z = sy;

                ui.input(|i| {
                    // Forward/backward — moves toward screen center (includes pitch)
                    if i.key_down(egui::Key::W) {
                        self.cam_x += front_x * actual_speed;
                        self.cam_y += front_y * actual_speed;
                        self.cam_z += front_z * actual_speed;
                    }
                    if i.key_down(egui::Key::S) {
                        self.cam_x -= front_x * actual_speed;
                        self.cam_y -= front_y * actual_speed;
                        self.cam_z -= front_z * actual_speed;
                    }
                    // Strafe left/right — always horizontal
                    if i.key_down(egui::Key::A) {
                        self.cam_x -= right_x * actual_speed;
                        self.cam_z -= right_z * actual_speed;
                    }
                    if i.key_down(egui::Key::D) {
                        self.cam_x += right_x * actual_speed;
                        self.cam_z += right_z * actual_speed;
                    }
                    // Up: E or Space — world up
                    if i.key_down(egui::Key::E) || i.key_down(egui::Key::Space) {
                        self.cam_y += actual_speed;
                    }
                    // Down: Q — world down
                    if i.key_down(egui::Key::Q) {
                        self.cam_y -= actual_speed;
                    }
                });

                // Scroll adjusts FPS base speed
                let scroll = ui.input(|i| i.raw_scroll_delta.y);
                if scroll != 0.0 {
                    self.fps_speed *= 1.0 + scroll * 0.002;
                    self.fps_speed = self.fps_speed.clamp(0.005, 2.0);
                }
            }
        } else {
            // ── ORBITAL MODE ──

            // Determine orbit drag: MMB (no shift/ctrl) OR LMB+Alt (no shift) OR LMB (no modifiers)
            let mmb_orbit = response.dragged_by(egui::PointerButton::Middle) && !shift && !ctrl;
            let alt_orbit = response.dragged_by(egui::PointerButton::Primary) && alt && !shift;
            let lmb_orbit = response.dragged_by(egui::PointerButton::Primary) && !alt && !shift && !ctrl;

            if mmb_orbit || alt_orbit || lmb_orbit {
                let delta = response.drag_delta();
                self.rotation_y += delta.x * ORBIT_SENSITIVITY;
                self.rotation_x += delta.y * ORBIT_SENSITIVITY;
                // No elevation clamp — allow endless rotation in all directions
                self.state = InteractionState::Orbiting;
            }

            // Determine pan drag: Shift+MMB OR Alt+Shift+LMB OR Shift+LMB
            let mmb_pan = response.dragged_by(egui::PointerButton::Middle) && shift && !ctrl;
            let alt_pan = response.dragged_by(egui::PointerButton::Primary) && alt && shift;
            let lmb_pan = response.dragged_by(egui::PointerButton::Primary) && shift && !alt && !ctrl;

            if mmb_pan || alt_pan {
                let delta = response.drag_delta();
                // Pan speed scales with distance (approximated by 1/zoom)
                let pan_factor = 1.0 / self.zoom;
                self.pan_x += delta.x * pan_factor;
                self.pan_y += delta.y * pan_factor;
                self.state = InteractionState::Panning;
            } else if lmb_pan {
                // Backward-compat: Shift+LMB pan with 1:1 pixel mapping
                let delta = response.drag_delta();
                self.pan_x += delta.x;
                self.pan_y += delta.y;
                self.state = InteractionState::Panning;
            }

            // Ctrl + MMB drag = smooth zoom
            if response.dragged_by(egui::PointerButton::Middle) && ctrl {
                let delta = response.drag_delta();
                self.zoom *= 1.0 + delta.y * -0.005;
                self.zoom = self.zoom.clamp(0.1, 50.0);
                self.state = InteractionState::Zooming;
            }

            // Scroll to zoom toward cursor
            if response.hovered() {
                let scroll = ui.input(|i| i.raw_scroll_delta.y);
                if scroll != 0.0 {
                    let old_zoom = self.zoom;
                    self.zoom *= 1.0 + scroll * 0.002;
                    self.zoom = self.zoom.clamp(0.1, 50.0);

                    // Adjust pan so the point under the cursor stays fixed
                    let mouse_pos = ui.input(|i| i.pointer.hover_pos().unwrap_or(response.rect.center()));
                    let center = response.rect.center();
                    let cursor_rel_x = mouse_pos.x - center.x - self.pan_x;
                    let cursor_rel_y = mouse_pos.y - center.y - self.pan_y;
                    let zoom_ratio = self.zoom / old_zoom;
                    self.pan_x += cursor_rel_x * (1.0 - zoom_ratio);
                    self.pan_y += cursor_rel_y * (1.0 - zoom_ratio);
                }
            }

            // Reset drag state when not dragging
            if !response.dragged() {
                self.state = InteractionState::Idle;
            }
        }

        // ── KEYBOARD SHORTCUTS (both modes, when hovered) ──
        if response.hovered() {
            ui.input(|i| {
                // Numpad view presets (Num1/3/7) + Ctrl variants
                if i.key_pressed(egui::Key::Num1) {
                    if ctrl {
                        self.set_view(0.0, std::f32::consts::PI); // Back
                    } else {
                        self.set_view(0.0, 0.0); // Front
                    }
                }
                if i.key_pressed(egui::Key::Num3) {
                    if ctrl {
                        self.set_view(0.0, -std::f32::consts::FRAC_PI_2); // Left
                    } else {
                        self.set_view(0.0, std::f32::consts::FRAC_PI_2); // Right
                    }
                }
                if i.key_pressed(egui::Key::Num7) {
                    if ctrl {
                        self.set_view(-std::f32::consts::FRAC_PI_2, 0.0); // Bottom
                    } else {
                        self.set_view(std::f32::consts::FRAC_PI_2, 0.0); // Top
                    }
                }
                // Numpad 5 = toggle ortho/perspective
                if i.key_pressed(egui::Key::Num5) {
                    self.perspective = !self.perspective;
                }
                // Home = reset view
                if i.key_pressed(egui::Key::Home) {
                    self.reset_view();
                }
            });
        }

        // Double-click to reset view
        if response.double_clicked() {
            self.reset_view();
        }
    }

    /// Set orbital view to the given rotation angles, resetting pan.
    pub fn set_view(&mut self, rotation_x: f32, rotation_y: f32) {
        self.rotation_x = rotation_x;
        self.rotation_y = rotation_y;
        self.pan_x = 0.0;
        self.pan_y = 0.0;
    }

    /// Reset all camera state to defaults.
    pub fn reset_view(&mut self) {
        self.rotation_x = 0.3;
        self.rotation_y = 0.4;
        self.zoom = 1.0;
        self.pan_x = 0.0;
        self.pan_y = 0.0;
        self.cam_x = 0.0;
        self.cam_y = 0.0;
        self.cam_z = -3.0;
        self.cam_yaw = 0.0;
        self.cam_pitch = 0.0;
    }

    // ══════════════════════════════════════════════════════════════════════════
    // ── BEGIN (allocate painter + handle input) ──
    // ══════════════════════════════════════════════════════════════════════════

    /// Allocate a painter area and handle 3D interaction. Returns (Response, Painter, Rect).
    pub fn begin(&mut self, ui: &mut egui::Ui) -> (egui::Response, egui::Painter, egui::Rect) {
        let available = ui.available_size();
        let (response, painter) = ui.allocate_painter(
            egui::Vec2::new(available.x, available.y),
            egui::Sense::click_and_drag(),
        );
        self.handle_input(&response, ui);
        if self.fps_mode {
            ui.ctx().request_repaint();
        }
        let rect = response.rect;
        (response, painter, rect)
    }

    // ══════════════════════════════════════════════════════════════════════════
    // ── ORIGIN GIZMO (colored axis lines at origin) ──
    // ══════════════════════════════════════════════════════════════════════════

    /// Draw a colored XYZ axis gizmo at a given 3D position.
    /// `pos`: 3D position of the gizmo origin (e.g. hub center).
    /// `axis_len`: length of each axis line in 3D units.
    pub fn draw_origin_gizmo_at(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        center: egui::Pos2,
        scale: f32,
        pos: [f32; 3],
        axis_len: f32,
    ) {
        if !self.show_origin_gizmo {
            return;
        }

        let origin = self.project(pos[0], pos[1], pos[2], center, scale);
        let x_tip = self.project(pos[0] + axis_len, pos[1], pos[2], center, scale);
        let y_tip = self.project(pos[0], pos[1] + axis_len, pos[2], center, scale);
        let z_tip = self.project(pos[0], pos[1], pos[2] + axis_len, center, scale);

        let width = 1.5;
        let font = egui::FontId::proportional(10.0);

        if rect.contains(origin) {
            painter.line_segment([origin, x_tip], egui::Stroke::new(width, AXIS_X_COLOR));
            painter.line_segment([origin, y_tip], egui::Stroke::new(width, AXIS_Y_COLOR));
            painter.line_segment([origin, z_tip], egui::Stroke::new(width, AXIS_Z_COLOR));

            if rect.contains(x_tip) {
                painter.text(x_tip, egui::Align2::LEFT_BOTTOM, "X", font.clone(), AXIS_X_COLOR);
            }
            if rect.contains(y_tip) {
                painter.text(y_tip, egui::Align2::LEFT_BOTTOM, "Y", font.clone(), AXIS_Y_COLOR);
            }
            if rect.contains(z_tip) {
                painter.text(z_tip, egui::Align2::LEFT_BOTTOM, "Z", font, AXIS_Z_COLOR);
            }
        }
    }

    /// Draw a colored XYZ axis gizmo at the 3D origin (0,0,0) with unit-length axes.
    pub fn draw_origin_gizmo(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        center: egui::Pos2,
        scale: f32,
    ) {
        self.draw_origin_gizmo_at(painter, rect, center, scale, [0.0, 0.0, 0.0], 1.0);
    }

    // ══════════════════════════════════════════════════════════════════════════
    // ── GRID DRAWING ──
    // ══════════════════════════════════════════════════════════════════════════

    /// Draw a back grid and bottom grid for the 3D scene (backward-compatible).
    pub fn draw_grid(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        theme: &super::Theme,
        center: egui::Pos2,
        scale: f32,
        z_back: f32,
    ) {
        let gc = theme.grid_color();
        // Back grid — vertical lines
        for g in 0..11 {
            let gx = -1.5 + (g as f32 / 10.0) * 3.0;
            let p1 = self.project(gx, -1.0, z_back, center, scale);
            let p2 = self.project(gx, 1.0, z_back, center, scale);
            if rect.contains(p1) && rect.contains(p2) {
                painter.line_segment([p1, p2], egui::Stroke::new(0.5, gc));
            }
        }
        // Back grid — horizontal lines
        for g in 0..9 {
            let gy = -1.0 + (g as f32 / 8.0) * 2.0;
            let p1 = self.project(-1.5, gy, z_back, center, scale);
            let p2 = self.project(1.5, gy, z_back, center, scale);
            if rect.contains(p1) && rect.contains(p2) {
                painter.line_segment([p1, p2], egui::Stroke::new(0.5, gc));
            }
        }
        // Bottom grid — depth lines
        for g in 0..11 {
            let gx = -1.5 + (g as f32 / 10.0) * 3.0;
            let p1 = self.project(gx, -1.0, z_back, center, scale);
            let p2 = self.project(gx, -1.0, -z_back, center, scale);
            if rect.contains(p1) && rect.contains(p2) {
                painter.line_segment([p1, p2], egui::Stroke::new(0.5, gc));
            }
        }
    }

    /// Draw an XZ plane grid (spec-compliant).
    /// Subtle gray lines every 1 unit, bold every 10 units.
    pub fn draw_xz_grid(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        center: egui::Pos2,
        scale: f32,
        half_extent: f32,
        grid_y: f32,
    ) {
        let subtle = egui::Color32::from_rgb(0x44, 0x44, 0x44);
        let bold = egui::Color32::from_rgb(0x66, 0x66, 0x66);

        let start = -(half_extent as i32);
        let end = half_extent as i32;

        for i in start..=end {
            let f = i as f32;
            let is_bold = i % 10 == 0;
            let color = if is_bold { bold } else { subtle };
            let width = if is_bold { 1.0 } else { 0.5 };

            // Z-parallel lines (varying X)
            let p1 = self.project(f, grid_y, start as f32, center, scale);
            let p2 = self.project(f, grid_y, end as f32, center, scale);
            if rect.contains(p1) || rect.contains(p2) {
                painter.line_segment([p1, p2], egui::Stroke::new(width, color));
            }

            // X-parallel lines (varying Z)
            let p1 = self.project(start as f32, grid_y, f, center, scale);
            let p2 = self.project(end as f32, grid_y, f, center, scale);
            if rect.contains(p1) || rect.contains(p2) {
                painter.line_segment([p1, p2], egui::Stroke::new(width, color));
            }
        }
    }

    // ══════════════════════════════════════════════════════════════════════════
    // ── DRAWING HELPERS ──
    // ══════════════════════════════════════════════════════════════════════════

    /// Draw axis labels at the edges of the 3D scene.
    pub fn draw_axes_labels(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        theme: &super::Theme,
        center: egui::Pos2,
        scale: f32,
        z_back: f32,
        labels: [&str; 3],
    ) {
        let tc = theme.text_color();
        let font = egui::FontId::proportional(14.0);

        // X axis label (bottom center)
        let px = self.project(0.0, -1.4, z_back, center, scale);
        if rect.contains(px) {
            painter.text(px, egui::Align2::CENTER_TOP, labels[0], font.clone(), tc);
        }
        // Y axis label (left side)
        let py = self.project(-1.8, 0.0, z_back, center, scale);
        if rect.contains(py) {
            painter.text(py, egui::Align2::RIGHT_CENTER, labels[1], font.clone(), tc);
        }
        // Z axis label (bottom right)
        let pz = self.project(1.6, -1.4, 0.0, center, scale);
        if rect.contains(pz) {
            painter.text(pz, egui::Align2::CENTER_TOP, labels[2], font, tc);
        }
    }

    /// Draw a 3D polyline (list of [x, y, z] points) with the given color and width.
    pub fn draw_line_3d(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        center: egui::Pos2,
        scale: f32,
        points: &[[f32; 3]],
        color: egui::Color32,
        width: f32,
    ) {
        let projected: Vec<egui::Pos2> = points
            .iter()
            .map(|p| self.project(p[0], p[1], p[2], center, scale))
            .collect();
        for i in 0..projected.len().saturating_sub(1) {
            if rect.contains(projected[i]) && rect.contains(projected[i + 1]) {
                painter.line_segment(
                    [projected[i], projected[i + 1]],
                    egui::Stroke::new(width, color),
                );
            }
        }
    }

    /// Draw a filled area between a 3D polyline and a base plane (y=base_y).
    pub fn draw_filled_3d(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        center: egui::Pos2,
        scale: f32,
        points: &[[f32; 3]],
        base_y: f32,
        color: egui::Color32,
    ) {
        let fill = color.linear_multiply(0.15);
        for i in 0..points.len().saturating_sub(1) {
            let pa = self.project(points[i][0], points[i][1], points[i][2], center, scale);
            let pb = self.project(points[i + 1][0], points[i + 1][1], points[i + 1][2], center, scale);
            let ba = self.project(points[i][0], base_y, points[i][2], center, scale);
            let bb = self.project(points[i + 1][0], base_y, points[i + 1][2], center, scale);
            if rect.contains(pa) && rect.contains(pb) {
                painter.add(egui::Shape::convex_polygon(
                    vec![pa, pb, bb, ba],
                    fill,
                    egui::Stroke::NONE,
                ));
            }
        }
    }
}
