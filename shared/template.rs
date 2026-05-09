// ╔══════════════════════════════════════════════════════════════════════════════╗
// ║                IWES / LUH — Shared egui App Module                        ║
// ║                                                                           ║
// ║  This file is a shared module imported by app scripts via:                ║
// ║      #[path = "../shared/template.rs"] mod template; (flat apps)          ║
// ║      #[path = "../../shared/template.rs"] mod template; (sub-apps)        ║
// ║      use template::*;                                                     ║
// ║                                                                           ║
// ║  It provides: theming, logo header bar, about dialog, 3D view helpers,   ║
// ║  UI helpers, and the IwesApp trait with run_app() launcher.               ║
// ║  DO NOT add a main() or shebang here — this is a library module.         ║
// ║                                                                           ║
// ║  HOW TO CREATE A NEW APP:                                                 ║
// ║  1. Create a new .rs file in apps/ (or apps/myapp/mod.rs)                 ║
// ║  2. Add a [[bin]] entry in Cargo.toml pointing at apps/myapp.rs          ║
// ║  3. Import: #[path = "../shared/template.rs"] mod template;               ║
// ║  4. Add at the top of your file (suppresses CMD window on Windows):      ║
// ║       #![cfg_attr(target_os = "windows", windows_subsystem = "windows")] ║
// ║  5. Define your struct with an AppBase field                              ║
// ║  6. Implement IwesApp trait (title, base, base_mut, controls, content)   ║
// ║  7. Call run_app::<MyApp>("Title", [width, height]) in main()            ║
// ║  8. Build: cargo build --release --bin my_app                             ║
// ║                                                                           ║
// ║  EXAMPLE APP (minimal, in apps/my_app.rs):                               ║
// ║  ┌────────────────────────────────────────────────────────────────────┐   ║
// ║  │ // apps/my_app.rs                                                 │   ║
// ║  │ #[path = "../shared/template.rs"] mod template;                   │   ║
// ║  │ use template::*;                                                  │   ║
// ║  │ use eframe::egui;                                                 │   ║
// ║  │                                                                   │   ║
// ║  │ struct MyApp { base: AppBase, value: f64 }                        │   ║
// ║  │ impl Default for MyApp {                                          │   ║
// ║  │     fn default() -> Self {                                        │   ║
// ║  │         Self { base: AppBase::default(), value: 1.0 }             │   ║
// ║  │     }                                                             │   ║
// ║  │ }                                                                 │   ║
// ║  │ impl IwesApp for MyApp {                                          │   ║
// ║  │     fn title(&self) -> &str { "My App" }                          │   ║
// ║  │     fn base(&self) -> &AppBase { &self.base }                     │   ║
// ║  │     fn base_mut(&mut self) -> &mut AppBase { &mut self.base }     │   ║
// ║  │     fn controls(&mut self, ui: &mut egui::Ui) {                   │   ║
// ║  │         labeled_slider(ui, "Value:", &mut self.value,             │   ║
// ║  │                        0.0..=10.0, "");                           │   ║
// ║  │     }                                                             │   ║
// ║  │     fn content(&mut self, ui: &mut egui::Ui) {                    │   ║
// ║  │         ui.label(format!("Value = {:.2}", self.value));           │   ║
// ║  │     }                                                             │   ║
// ║  │ }                                                                 │   ║
// ║  │ fn main() -> eframe::Result<()> {                                 │   ║
// ║  │     run_app::<MyApp>("My App", [1400.0, 900.0])                   │   ║
// ║  │ }                                                                 │   ║
// ║  └────────────────────────────────────────────────────────────────────┘   ║
// ║                                                                           ║
// ║  INSTRUCTIONS FOR CLAUDE CODE:                                            ║
// ║  - When creating a new IWES app, import this module and implement         ║
// ║    the IwesApp trait. Do NOT copy-paste the template contents.            ║
// ║  - The app frontmatter MUST include: eframe, egui_plot, image.           ║
// ║  - Use labeled_slider(), section_header() for controls.                   ║
// ║  - Use base().theme for colors, base_mut().view_3d for 3D views.        ║
// ║  - Access time_offset via base().time_offset for animation.              ║
// ║  - Do NOT modify this file unless explicitly asked.                       ║
// ║                                                                           ║
// ║  OPTIONAL DEPENDENCIES (add to app frontmatter as needed):               ║
// ║    nalgebra = "0.33"          # Linear algebra, vectors, matrices         ║
// ║    ndarray = "0.16"           # N-dimensional arrays                      ║
// ║    rustfft = "6.2"            # Fast Fourier Transform                    ║
// ║    serde = { version = "1", features = ["derive"] }  # Serialization     ║
// ║    serde_json = "1"           # JSON save/load                            ║
// ║    csv = "1.3"                # CSV data import/export                    ║
// ║    egui_extras = { version = "0.29", features = ["image"] }              ║
// ║                                                                           ║
// ║  EXPORTED TYPES AND FUNCTIONS:                                            ║
// ║    AppBase       — shared state (theme, view_3d, animate, time_offset)   ║
// ║    IwesApp       — trait to implement for your app                        ║
// ║    run_app()     — one-liner to launch your app                           ║
// ║    Theme         — dark/light mode, color getters                         ║
// ║    View3D        — 3D projection, rotation, zoom, grid drawing           ║
// ║    labeled_slider(ui, label, value, range, suffix)                       ║
// ║    section_header(ui, title)                                              ║
// ║                                                                           ║
// ║  Author: Basem Rajjoub, 2026 — LUH / IWES                                ║
// ╚══════════════════════════════════════════════════════════════════════════════╝

#![allow(dead_code)]

use eframe::egui;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys;

mod viewport3d;
pub use viewport3d::*;

// ══════════════════════════════════════════════════════════════════════════════
// ── EMBEDDED ASSETS ──
// ══════════════════════════════════════════════════════════════════════════════

pub const LUH_LOGO_BYTES: &[u8] = include_bytes!("figures/01-LUH-Logo.png");
pub const IWES_LOGO_BYTES: &[u8] = include_bytes!("figures/02-IWES-Logo.png");

/// Decode a PNG from bytes into an egui TextureHandle (call once, cache the result).
pub fn load_texture_from_png(ctx: &egui::Context, name: &str, png_bytes: &[u8]) -> egui::TextureHandle {
    let img = image::load_from_memory(png_bytes).expect("Failed to decode embedded PNG");
    let rgba = img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let pixels = rgba.into_raw();
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
    ctx.load_texture(name, color_image, egui::TextureOptions::LINEAR)
}

// ══════════════════════════════════════════════════════════════════════════════
// ── THEME SYSTEM ──
// ══════════════════════════════════════════════════════════════════════════════

pub struct Theme {
    pub dark_mode: bool,
}

impl Default for Theme {
    fn default() -> Self {
        Self { dark_mode: false }
    }
}

impl Theme {
    /// Apply the theme visuals to the egui context. Call once per frame.
    pub fn apply(&self, ctx: &egui::Context) {
        if self.dark_mode {
            ctx.set_visuals(egui::Visuals::dark());
        } else {
            ctx.set_visuals(egui::Visuals::light());
        }
    }

    pub fn bg_color(&self) -> egui::Color32 {
        if self.dark_mode {
            egui::Color32::from_rgb(20, 20, 30)
        } else {
            egui::Color32::from_rgb(240, 240, 245)
        }
    }

    pub fn text_color(&self) -> egui::Color32 {
        if self.dark_mode {
            egui::Color32::from_rgb(200, 200, 210)
        } else {
            egui::Color32::from_rgb(40, 40, 50)
        }
    }

    pub fn grid_color(&self) -> egui::Color32 {
        if self.dark_mode {
            egui::Color32::from_rgba_premultiplied(60, 60, 80, 100)
        } else {
            egui::Color32::from_rgba_premultiplied(150, 150, 170, 100)
        }
    }

    pub fn accent_color(&self) -> egui::Color32 {
        egui::Color32::from_rgb(50, 150, 230)
    }

    pub fn header_bg(&self) -> egui::Color32 {
        if self.dark_mode {
            egui::Color32::from_rgb(22, 25, 38)
        } else {
            egui::Color32::WHITE
        }
    }

    pub fn composite_color(&self) -> egui::Color32 {
        if self.dark_mode {
            egui::Color32::WHITE
        } else {
            egui::Color32::from_rgb(20, 20, 20)
        }
    }

    /// A palette of distinct colors for multi-series data.
    pub const SERIES_COLORS: [egui::Color32; 6] = [
        egui::Color32::from_rgb(230, 60, 70),    // red
        egui::Color32::from_rgb(140, 60, 220),    // purple
        egui::Color32::from_rgb(50, 150, 230),    // blue
        egui::Color32::from_rgb(230, 130, 200),   // pink
        egui::Color32::from_rgb(80, 200, 120),    // green
        egui::Color32::from_rgb(255, 180, 50),    // orange
    ];

    pub fn series_color(&self, idx: usize) -> egui::Color32 {
        Self::SERIES_COLORS[idx % Self::SERIES_COLORS.len()]
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// ── PANEL CONFIGURATION (for apps with custom layouts) ──
// ══════════════════════════════════════════════════════════════════════════════

/// Tab definition for left panel tabs.
/// Apps can define multiple tabs to organize their controls.
#[derive(Clone)]
pub struct TabDef {
    pub label: &'static str,
}

impl TabDef {
    pub const fn new(label: &'static str) -> Self {
        Self { label }
    }
}

/// Right panel configuration for apps that need a third column.
#[derive(Clone)]
pub struct RightPanelConfig {
    pub width: f32,
    pub resizable: bool,
}

impl Default for RightPanelConfig {
    fn default() -> Self {
        Self {
            width: 320.0,
            resizable: true,
        }
    }
}

/// Optional panel configuration for apps needing custom layouts.
/// Apps that don't provide this get the default single left panel.
#[derive(Clone)]
pub struct PanelConfig {
    pub left_width: f32,
    pub right_panel: Option<RightPanelConfig>,
    pub left_tabs: Option<Vec<TabDef>>,
}

impl Default for PanelConfig {
    fn default() -> Self {
        Self {
            left_width: 260.0,
            right_panel: None,
            left_tabs: None,
        }
    }
}

// ── 3D View: see viewport3d.rs ──

// ══════════════════════════════════════════════════════════════════════════════
// ── UI HELPERS ──
// ══════════════════════════════════════════════════════════════════════════════

/// A labeled slider in a horizontal layout. Returns the slider response.
pub fn labeled_slider(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
    suffix: &str,
) -> egui::Response {
    let mut resp = None;
    ui.horizontal(|ui| {
        ui.label(label);
        resp = Some(ui.add(egui::Slider::new(value, range).suffix(suffix)));
    });
    resp.unwrap()
}

/// Same as `labeled_slider` but with an explicit drag step for finer/coarser control.
pub fn labeled_slider_step(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
    suffix: &str,
    step: f64,
) -> egui::Response {
    let mut resp = None;
    ui.horizontal(|ui| {
        ui.label(label);
        resp = Some(ui.add(egui::Slider::new(value, range).suffix(suffix).step_by(step)));
    });
    resp.unwrap()
}

/// Logarithmic slider — equal slider travel per decade. Ideal for wide positive ranges.
/// Tip: Ctrl+drag for 10× finer precision; click the value label to type directly.
pub fn labeled_slider_log(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
    suffix: &str,
) -> egui::Response {
    let mut resp = None;
    ui.horizontal(|ui| {
        ui.label(label);
        resp = Some(ui.add(
            egui::Slider::new(value, range)
                .suffix(suffix)
                .logarithmic(true)
                .min_decimals(2)
                .max_decimals(4),
        ));
    });
    resp.unwrap()
}

/// A section header: heading text + separator below.
pub fn section_header(ui: &mut egui::Ui, title: &str) {
    ui.separator();
    ui.heading(title);
    ui.separator();
}

/// Thin draggable vertical split handle (4 px wide, 1 px visible line).
/// Place inside a `ui.horizontal()` between two panels.
/// Returns the horizontal drag delta — add to your tracked split pixel position.
pub fn vsplit_handle(ui: &mut egui::Ui, h: f32) -> f32 {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(4.0, h), egui::Sense::drag());
    let col = if resp.hovered() || resp.dragged() {
        egui::Color32::from_gray(180)
    } else {
        egui::Color32::from_gray(100)
    };
    let cx = rect.center().x;
    ui.painter().rect_filled(
        egui::Rect::from_min_max(egui::pos2(cx - 0.5, rect.top()), egui::pos2(cx + 0.5, rect.bottom())),
        0.0, col,
    );
    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }
    resp.drag_delta().x
}

/// Thin draggable horizontal split handle (4 px tall, 1 px visible line).
/// Place inside a `ui.vertical()` between two sections.
/// Returns the vertical drag delta — add to your tracked split pixel position.
pub fn hsplit_handle(ui: &mut egui::Ui, w: f32) -> f32 {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, 4.0), egui::Sense::drag());
    let col = if resp.hovered() || resp.dragged() {
        egui::Color32::from_gray(180)
    } else {
        egui::Color32::from_gray(100)
    };
    let cy = rect.center().y;
    ui.painter().rect_filled(
        egui::Rect::from_min_max(egui::pos2(rect.left(), cy - 0.5), egui::pos2(rect.right(), cy + 0.5)),
        0.0, col,
    );
    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
    }
    resp.drag_delta().y
}

/// Draw the top header bar with LUH logo (left), app title (center), IWES logo + About (right).
fn logo_header(
    ui: &mut egui::Ui,
    luh_tex: &Option<egui::TextureHandle>,
    iwes_tex: &Option<egui::TextureHandle>,
    theme: &Theme,
    app_title: &str,
    show_about: &mut bool,
) {
    let header_height = 48.0;
    let (rect, _) = ui.allocate_exact_size(
        egui::Vec2::new(ui.available_width(), header_height),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, theme.header_bg());

    // IWES logo — left side
    if let Some(tex) = iwes_tex {
        let logo_h = header_height - 8.0;
        let aspect = tex.size()[0] as f32 / tex.size()[1] as f32;
        let logo_w = logo_h * aspect;
        let logo_rect = egui::Rect::from_min_size(
            egui::Pos2::new(rect.left() + 8.0, rect.top() + 4.0),
            egui::Vec2::new(logo_w, logo_h),
        );
        painter.image(
            tex.id(),
            logo_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }

    // App title — center
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        app_title,
        egui::FontId::proportional(20.0),
        theme.text_color(),
    );

    // LUH logo — right side (offset left for about button)
    if let Some(tex) = luh_tex {
        let logo_h = header_height - 12.0;
        let aspect = tex.size()[0] as f32 / tex.size()[1] as f32;
        let logo_w = logo_h * aspect;
        let logo_rect = egui::Rect::from_min_size(
            egui::Pos2::new(rect.right() - logo_w - 70.0, rect.top() + 6.0),
            egui::Vec2::new(logo_w, logo_h),
        );
        painter.image(
            tex.id(),
            logo_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }

    // About button — far right
    let btn_rect = egui::Rect::from_min_size(
        egui::Pos2::new(rect.right() - 58.0, rect.top() + 10.0),
        egui::Vec2::new(50.0, header_height - 20.0),
    );
    let btn_resp = ui.put(btn_rect, egui::Button::new("About"));
    if btn_resp.clicked() {
        *show_about = !*show_about;
    }
}

/// Show the About window as a modal dialog.
fn about_window(ctx: &egui::Context, open: &mut bool) {
    if !*open {
        return;
    }
    egui::Window::new("About")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(10.0);
                ui.heading("About This Application");
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);
                ui.label(egui::RichText::new("Created by").size(14.0));
                ui.label(egui::RichText::new("Basem Rajjoub").strong().size(18.0));
                ui.add_space(8.0);
                ui.label(egui::RichText::new("Leibniz Universit\u{00e4}t Hannover").size(14.0));
                ui.label(
                    egui::RichText::new("Institut f\u{00fc}r Windenergiesysteme (IWES)")
                        .size(14.0),
                );
                ui.add_space(8.0);
                ui.hyperlink_to(
                    "basem.rajjoub@iwes.hannover-uni.de",
                    "mailto:basem.rajjoub@iwes.hannover-uni.de",
                );
                ui.add_space(15.0);
                if ui.button("Close").clicked() {
                    *open = false;
                }
                ui.add_space(5.0);
            });
        });
}

// ══════════════════════════════════════════════════════════════════════════════
// ── APP BASE & TRAIT ──
// ══════════════════════════════════════════════════════════════════════════════

/// Shared state embedded in every IWES app. Contains theme, 3D view, animation,
/// logo textures, and the about dialog state.
pub struct AppBase {
    pub theme: Theme,
    pub view_3d: View3D,
    pub animate: bool,
    pub loop_animation: bool,
    pub time_offset: f64,
    pub t_max: f64,
    pub show_about: bool,
    /// Active tab index for apps using tabbed left panel
    pub active_left_tab: usize,
    /// Fractional split position [0..1] for apps with a resizable left/right split.
    /// Left column gets `content_split` fraction of available width.
    pub content_split: f32,
    luh_texture: Option<egui::TextureHandle>,
    iwes_texture: Option<egui::TextureHandle>,
}

impl Default for AppBase {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            view_3d: View3D::default(),
            animate: true,
            loop_animation: true,
            time_offset: 0.0,
            t_max: f64::INFINITY,
            show_about: false,
            active_left_tab: 0,
            content_split: 0.65,
            luh_texture: None,
            iwes_texture: None,
        }
    }
}

/// Trait that every IWES app implements. The framework handles logos, theme,
/// animation, about dialog, and panel layout — you only write controls() and content().
pub trait IwesApp: Default + 'static {
    /// The app title shown in the header bar.
    fn title(&self) -> &str;

    /// Return a reference to the embedded AppBase.
    fn base(&self) -> &AppBase;

    /// Return a mutable reference to the embedded AppBase.
    fn base_mut(&mut self) -> &mut AppBase;

    /// Draw your controls in the left side panel.
    /// Theme toggle and animation checkbox are already drawn above this.
    fn controls(&mut self, ui: &mut egui::Ui);

    /// Draw your main content in the central panel.
    /// Use Plot, View3D, custom painting, etc.
    fn content(&mut self, ui: &mut egui::Ui);

    // ── Optional methods for custom layouts (default implementations for backward compat) ──

    /// Return panel configuration for custom layouts. Default: standard single left panel.
    fn panel_config(&self) -> Option<PanelConfig> {
        None
    }

    /// Return tab definitions for left panel. Default: no tabs (single controls section).
    fn control_tabs(&self) -> Option<&[TabDef]> {
        None
    }

    /// Draw controls for a specific tab index. Default: calls controls() for tab 0.
    /// Override this when using tabs to draw different controls per tab.
    fn controls_for_tab(&mut self, ui: &mut egui::Ui, tab_index: usize) {
        if tab_index == 0 {
            self.controls(ui);
        }
    }

    /// Draw content in the right panel. Default: nothing.
    /// Override this when using a right panel (e.g., for equations).
    fn right_panel(&mut self, _ui: &mut egui::Ui) {}
}

/// Wrapper that implements eframe::App for any IwesApp.
pub struct AppWrapper<T: IwesApp> {
    pub app: T,
}

impl<T: IwesApp> eframe::App for AppWrapper<T> {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let base = self.app.base_mut();

        // Load logo textures on first frame
        if base.luh_texture.is_none() {
            base.luh_texture = Some(load_texture_from_png(ctx, "luh_logo", LUH_LOGO_BYTES));
        }
        if base.iwes_texture.is_none() {
            base.iwes_texture = Some(load_texture_from_png(ctx, "iwes_logo", IWES_LOGO_BYTES));
        }

        // Apply theme
        base.theme.apply(ctx);

        // Animation loop
        if base.animate {
            base.time_offset += 0.03;
            // Loop or stop at t_max
            if base.time_offset >= base.t_max {
                if base.loop_animation {
                    base.time_offset = 0.0;
                } else {
                    base.time_offset = base.t_max;
                    base.animate = false;
                }
            }
            ctx.request_repaint();
        }

        // About dialog
        about_window(ctx, &mut base.show_about);

        // Top header with logos
        let title = self.app.title().to_owned();
        let base = self.app.base_mut();
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            logo_header(
                ui,
                &base.luh_texture,
                &base.iwes_texture,
                &base.theme,
                &title,
                &mut base.show_about,
            );
        });

        // Get panel configuration (if any)
        let panel_config = self.app.panel_config();
        let left_width = panel_config.as_ref().map_or(260.0, |c| c.left_width);
        // Copy tab labels to avoid borrow conflicts in closures
        let tab_labels: Option<Vec<&'static str>> = self
            .app
            .control_tabs()
            .map(|tabs| tabs.iter().map(|t| t.label).collect());

        // Right panel (if configured) - must be added before central panel
        if let Some(ref config) = panel_config {
            if let Some(ref right_config) = config.right_panel {
                egui::SidePanel::right("right_panel")
                    .min_width(right_config.width)
                    .default_width(right_config.width)
                    .resizable(right_config.resizable)
                    .show(ctx, |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            self.app.right_panel(ui);
                        });
                    });
            }
        }

        // Left panel: framework controls + user controls (with optional tabs)
        egui::SidePanel::left("controls").min_width(left_width).resizable(true).show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Controls");
                ui.separator();

                // Theme toggle
                let base = self.app.base_mut();
                ui.horizontal(|ui| {
                    ui.label("Theme:");
                    if ui.selectable_label(base.theme.dark_mode, "Dark").clicked() {
                        base.theme.dark_mode = true;
                    }
                    if ui.selectable_label(!base.theme.dark_mode, "Light").clicked() {
                        base.theme.dark_mode = false;
                    }
                });

                // Animation toggle
                ui.horizontal(|ui| {
                    ui.checkbox(&mut base.animate, "Animate");
                    ui.checkbox(&mut base.loop_animation, "Loop");
                });
                ui.separator();

                // If tabs are defined, show tab bar and route to controls_for_tab
                if let Some(ref labels) = tab_labels {
                    let active_tab = self.app.base().active_left_tab;
                    ui.horizontal(|ui| {
                        for (i, label) in labels.iter().enumerate() {
                            let selected = active_tab == i;
                            if ui.selectable_label(selected, *label).clicked() {
                                self.app.base_mut().active_left_tab = i;
                            }
                        }
                    });
                    ui.separator();
                    let active_tab = self.app.base().active_left_tab;
                    self.app.controls_for_tab(ui, active_tab);
                } else {
                    // No tabs: just call controls() as before
                    self.app.controls(ui);
                }
            });
        });

        // Central panel: user content
        egui::CentralPanel::default().show(ctx, |ui| {
            self.app.content(ui);
        });
    }
}

/// Launch an IWES app. Call this from main().
/// Example: `run_app::<MyApp>("My App", [1400.0, 900.0])`
pub fn run_app<T: IwesApp>(title: &str, size: [f32; 2]) -> eframe::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size(size)
                .with_title(title),
            ..Default::default()
        };
        eframe::run_native(
            title,
            options,
            Box::new(|_cc| Ok(Box::new(AppWrapper { app: T::default() }))),
        )
    }
    #[cfg(target_arch = "wasm32")]
    {
        wasm_run_app::<T>(title, size)
    }
}

#[cfg(target_arch = "wasm32")]
fn wasm_run_app<T: IwesApp>(_title: &str, _size: [f32; 2]) -> eframe::Result<()> {
    console_error_panic_hook::set_once();
    let web_options = eframe::WebOptions::default();
    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .and_then(|w| w.document())
            .expect("Failed to get document");
        let canvas = document
            .create_element("canvas")
            .expect("Failed to create canvas")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("Failed to cast to HtmlCanvasElement");
        document.body().expect("Failed to get body")
            .append_child(&canvas)
            .expect("Failed to append canvas");

        let _ = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|_cc| Ok(Box::new(AppWrapper { app: T::default() }))),
            )
            .await;
    });
    Ok(())
}
