// SDOF Dynamics Explorer — IWES / LUH
// Build: cargo build --release   Run: cargo run --release
//
// m*x'' + c*x' + k*x = F0*cos(Omega*t)
//   omega_n = sqrt(k/m),  zeta = c/(2*sqrt(mk)),  r = Omega/omega_n

// Suppress the console window on Windows (GUI-only app)
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
#![allow(dead_code)]

#[path = "../shared/template.rs"]
mod template;
use template::*;

use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints, Points, VLine};
#[cfg(feature = "mathjax")]
use std::collections::HashMap;

const PI: f64 = std::f64::consts::PI;

// ── Palette for reference zeta curves ──
const ZETA_REFS: [f64; 5] = [0.05, 0.1, 0.2, 0.5, 1.0];
fn zeta_color(i: usize) -> egui::Color32 {
    match i {
        0 => egui::Color32::from_rgb( 60, 140, 220),
        1 => egui::Color32::from_rgb( 60, 190,  80),
        2 => egui::Color32::from_rgb(210, 170,  30),
        3 => egui::Color32::from_rgb(210, 100,  30),
        _ => egui::Color32::from_rgb(160,  60, 190),
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// ── MATHJAX PIPELINE ──
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "mathjax")]
struct RenderedEquation {
    texture: egui::TextureHandle,
    width: f32,
    height: f32,
}

#[cfg(feature = "mathjax")]
fn render_latex_to_texture(ctx: &egui::Context, latex: &str, name: &str, scale: f32, dark: bool)
    -> Option<RenderedEquation>
{
    let svg = match mathjax_svg::convert_to_svg(latex) {
        Ok(s) => s,
        Err(e) => { eprintln!("MathJax '{}': {:?}", name, e); return None; }
    };
    let svg = if dark {
        svg.replace("currentColor", "#c8c8d2").replace("color: ;", "color: #c8c8d2;")
    } else {
        svg.replace("currentColor", "#28282f")
    };
    let tree = match resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()) {
        Ok(t) => t,
        Err(e) => { eprintln!("SVG '{}': {:?}", name, e); return None; }
    };
    let sz = tree.size().to_int_size();
    let (w, h) = (((sz.width()  as f32)*scale) as u32,
                  ((sz.height() as f32)*scale) as u32);
    if w == 0 || h == 0 { return None; }
    let mut px = resvg::tiny_skia::Pixmap::new(w, h)?;
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut px.as_mut());
    let img = egui::ColorImage::from_rgba_premultiplied([w as usize, h as usize], px.data());
    Some(RenderedEquation {
        texture: ctx.load_texture(name, img, egui::TextureOptions::LINEAR),
        width: w as f32 / scale,
        height: h as f32 / scale,
    })
}

#[cfg(feature = "mathjax")]
struct LatexCache { entries: HashMap<String, RenderedEquation> }
#[cfg(feature = "mathjax")]
impl LatexCache {
    fn new() -> Self { Self { entries: HashMap::new() } }
    fn get_or_render(&mut self, key: &str, ctx: &egui::Context, latex: &str, scale: f32, dark: bool)
        -> Option<&RenderedEquation>
    {
        if !self.entries.contains_key(key) {
            if let Some(r) = render_latex_to_texture(ctx, latex, key, scale, dark) {
                self.entries.insert(key.to_string(), r);
            }
        }
        self.entries.get(key)
    }
    fn invalidate(&mut self) { self.entries.clear(); }
}

#[cfg(not(feature = "mathjax"))]
struct LatexCache;
#[cfg(not(feature = "mathjax"))]
impl LatexCache {
    fn new() -> Self { Self }
    fn get_or_render(&mut self, _key: &str, _ctx: &egui::Context, _latex: &str, _scale: f32, _dark: bool)
        -> Option<&()> { None }
    fn invalidate(&mut self) {}
}

#[cfg(feature = "mathjax")]
fn cv(v: f64, d: usize) -> String { format!(r"{{\color{{RoyalBlue}}{:.prec$}}}", v, prec=d) }

#[cfg(not(feature = "mathjax"))]
fn cv(v: f64, d: usize) -> String { format!("{:.prec$}", v, prec=d) }

fn hash_f64s(vals: &[f64]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for v in vals { h ^= v.to_bits(); h = h.wrapping_mul(0x100000001b3); }
    h
}

// ══════════════════════════════════════════════════════════════════════════════
// ── PHYSICS ──
// ══════════════════════════════════════════════════════════════════════════════

fn natural_freq(m: f64, k: f64) -> f64 { (k / m).sqrt() }
fn damping_ratio(m: f64, k: f64, c: f64) -> f64 { c / (2.0 * (m * k).sqrt()) }
fn damped_freq(wn: f64, zeta: f64) -> f64 {
    if zeta < 1.0 { wn * (1.0 - zeta*zeta).sqrt() } else { 0.0 }
}

/// H(r) — capped at 10 to keep the FRF plot readable
fn amplification(r: f64, zeta: f64) -> f64 {
    let d = ((1.0 - r*r).powi(2) + (2.0*zeta*r).powi(2)).sqrt();
    if d < 1e-12 { 10.0 } else { (1.0/d).min(10.0) }
}

/// Format a float, trimming insignificant trailing zeros (e.g. 0.010 → "0.01", 1.0 → "1").
fn fmt_trim(v: f64) -> String {
    let s = format!("{:.4}", v);
    let s = s.trim_end_matches('0');
    s.trim_end_matches('.').to_string()
}

fn phase_lag(r: f64, zeta: f64) -> f64 { f64::atan2(2.0*zeta*r, 1.0 - r*r) }

/// RK4 step: m*x'' + c*x' + k*x = F0*cos(Omega*t)
fn rk4_step(s: [f64;2], t: f64, dt: f64, m: f64, k: f64, c: f64, f0: f64, omega: f64) -> [f64;2] {
    let f = |s: [f64;2], ti: f64| -> [f64;2] {
        [s[1], (f0*(omega*ti).cos() - c*s[1] - k*s[0]) / m]
    };
    let k1 = f(s, t);
    let k2 = f([s[0]+0.5*dt*k1[0], s[1]+0.5*dt*k1[1]], t+0.5*dt);
    let k3 = f([s[0]+0.5*dt*k2[0], s[1]+0.5*dt*k2[1]], t+0.5*dt);
    let k4 = f([s[0]+    dt*k3[0], s[1]+    dt*k3[1]], t+    dt);
    [s[0]+dt/6.0*(k1[0]+2.0*k2[0]+2.0*k3[0]+k4[0]),
     s[1]+dt/6.0*(k1[1]+2.0*k2[1]+2.0*k3[1]+k4[1])]
}

struct TimeResponse { t: Vec<f64>, x: Vec<f64>, v: Vec<f64> }

fn compute_time_response(m: f64, k: f64, c: f64, f0: f64, omega: f64,
                         x0: f64, v0: f64, t_end: f64) -> TimeResponse
{
    let n = 3000_usize.min((t_end / 0.005).ceil() as usize + 1).max(3);
    let dt = t_end / (n-1) as f64;
    let mut tv = Vec::with_capacity(n);
    let mut xv = Vec::with_capacity(n);
    let mut vv = Vec::with_capacity(n);
    let mut s = [x0, v0];
    let mut t = 0.0;
    tv.push(t); xv.push(s[0]); vv.push(s[1]);
    for _ in 1..n {
        s = rk4_step(s, t, dt, m, k, c, f0, omega);
        t += dt;
        tv.push(t); xv.push(s[0]); vv.push(s[1]);
    }
    TimeResponse { t: tv, x: xv, v: vv }
}

/// Linear interpolation into a cached response at time t (wraps around t_end)
fn interpolate_response(resp: &TimeResponse, t: f64, t_end: f64) -> (f64, f64) {
    let t = t % t_end.max(1e-9);
    let n = resp.t.len();
    if n < 2 { return (0.0, 0.0); }
    let frac = (t / t_end) * (n-1) as f64;
    let i = (frac as usize).min(n-2);
    let a = frac - i as f64;
    let x = resp.x[i] + a * (resp.x[i+1] - resp.x[i]);
    let v = resp.v[i] + a * (resp.v[i+1] - resp.v[i]);
    (x, v)
}

fn compute_frf_curve(zeta: f64) -> Vec<[f64;2]> {
    (0..=600).map(|i| {
        let r = 3.0 * i as f64 / 600.0;
        [r, amplification(r, zeta)]
    }).collect()
}

fn compute_phase_curve(zeta: f64) -> Vec<[f64;2]> {
    (0..=600).map(|i| {
        let r = 3.0 * i as f64 / 600.0;
        [r, phase_lag(r, zeta).to_degrees()]
    }).collect()
}

fn detect_case(zeta: f64, f0: f64, r: f64) -> &'static str {
    if f0 < 1e-6 {
        if zeta < 1e-6             { "Undamped Free" }
        else if (zeta-1.0).abs() < 0.02 { "Critically Damped" }
        else if zeta < 1.0         { "Underdamped Free" }
        else                       { "Overdamped Free" }
    } else {
        if (r-1.0).abs() < 0.05 && zeta < 0.1 { "Resonance" }
        else                                    { "Forced Vibration" }
    }
}

fn case_color(case: &str) -> egui::Color32 {
    match case {
        "Undamped Free"     => egui::Color32::from_rgb(80, 200, 80),
        "Underdamped Free"  => egui::Color32::from_rgb(80, 150, 220),
        "Critically Damped" => egui::Color32::from_rgb(210, 170, 40),
        "Overdamped Free"   => egui::Color32::from_rgb(210, 110, 40),
        "Resonance"         => egui::Color32::from_rgb(230, 40, 40),
        _                   => egui::Color32::from_rgb(200, 70, 40),
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// ── APP STRUCT ──
// ══════════════════════════════════════════════════════════════════════════════

struct SdofApp {
    base:        AppBase,
    mass:        f64,
    stiffness:   f64,
    damping:     f64,
    force_amp:   f64,
    excit_freq:  f64,   // big Omega
    x0:          f64,
    v0:          f64,
    t_end:       f64,
    show_velocity: bool,
    show_envelope: bool,
    show_force:    bool,
    render_scale:  f32,
    display_scale: f32,
    // Per-preset context note shown in theory panel
    preset_note:   &'static str,
    // Resizable row fractions for the left column
    draw_h_frac:   f32,   // drawing / total resizable height
    time_h_frac:   f32,   // time plot / (total - drawing)
    // LaTeX cache
    cache:         LatexCache,
    last_dark:     bool,
    // RK4 response cache (for animation consistency with plot)
    resp_cache:    Option<TimeResponse>,
    resp_hash:     u64,
    // Drawing zoom / pan
    draw_zoom:     f32,
    draw_pan:      egui::Vec2,
}

impl Default for SdofApp {
    fn default() -> Self {
        let mut base = AppBase::default();
        base.t_max = 30.0;
        base.animate = true;
        Self {
            base,
            mass:        1.0,
            stiffness:   100.0,
            damping:     2.0,
            force_amp:   0.0,
            excit_freq:  8.0,
            x0:          0.1,
            v0:          0.0,
            t_end:       30.0,
            show_velocity: false,
            show_envelope: true,
            show_force:    false,
            render_scale:  4.0,
            display_scale: 1.0,
            preset_note:   "",
            draw_h_frac:   0.38,
            time_h_frac:   0.52,
            cache:         LatexCache::new(),
            last_dark:     false,
            resp_cache:    None,
            resp_hash:     0,
            draw_zoom:     1.0,
            draw_pan:      egui::Vec2::ZERO,
        }
    }
}

impl SdofApp {
    fn wn(&self)   -> f64 { natural_freq(self.mass, self.stiffness) }
    fn zeta(&self) -> f64 { damping_ratio(self.mass, self.stiffness, self.damping) }
    fn wd(&self)   -> f64 { damped_freq(self.wn(), self.zeta()) }
    fn r(&self)    -> f64 { let wn = self.wn(); if wn > 1e-9 { self.excit_freq/wn } else { 0.0 } }

    /// Ensure cached RK4 response is up-to-date
    fn ensure_resp(&mut self) {
        let h = hash_f64s(&[self.mass, self.stiffness, self.damping,
                             self.force_amp, self.excit_freq,
                             self.x0, self.v0, self.t_end]);
        if self.resp_hash != h || self.resp_cache.is_none() {
            self.resp_cache = Some(compute_time_response(
                self.mass, self.stiffness, self.damping,
                self.force_amp, self.excit_freq,
                self.x0, self.v0, self.t_end));
            self.resp_hash = h;
        }
    }

    /// x(t) from cached RK4 — exact match with the time plot
    fn x_anim(&self, t: f64) -> f64 {
        if let Some(r) = &self.resp_cache {
            let (x, _) = interpolate_response(r, t, self.t_end);
            x
        } else { 0.0 }
    }

    // ── Schematic (with zoom + pan) ────────────────────────────────────────
    fn draw_schematic(&mut self, ui: &mut egui::Ui) {
        let avail = ui.available_size();
        // Sense drag (pan) and hover (zoom via scroll)
        let (resp, painter) = ui.allocate_painter(avail, egui::Sense::click_and_drag());
        let rect = resp.rect;

        let theme  = &self.base.theme;
        let text_c = theme.text_color();
        let accent = theme.accent_color();
        let grid_c = theme.grid_color();
        painter.rect_filled(rect, 0.0, theme.bg_color());

        // ── Zoom / Pan interaction ──
        // Pan: left-drag
        if resp.dragged_by(egui::PointerButton::Primary) {
            self.draw_pan += resp.drag_delta();
        }
        // Zoom: scroll wheel (centered on mouse position)
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 && resp.hovered() {
            let old_zoom = self.draw_zoom;
            let new_zoom = (old_zoom * (1.0 + scroll * 0.004)).clamp(0.08, 20.0);
            if let Some(mp) = ui.input(|i| i.pointer.hover_pos()) {
                // Keep the point under the mouse fixed
                let local = (mp.to_vec2() - rect.center().to_vec2() - self.draw_pan) / old_zoom;
                self.draw_pan += local * (old_zoom - new_zoom);
            }
            self.draw_zoom = new_zoom;
        }
        // Double-click: reset view
        if resp.double_clicked() {
            self.draw_zoom = 1.0;
            self.draw_pan  = egui::Vec2::ZERO;
        }

        // ── Coordinate transform ──
        // All "base" coordinates are in scaled units relative to rect.center().
        // screen = rect.center() + draw_pan + draw_zoom * base_pos
        let base_scale = rect.width().min(rect.height()) * 0.55;
        let zoom  = self.draw_zoom;
        let pan   = self.draw_pan;
        // Helper: transform a base-space point to screen space
        let p = |bx: f32, by: f32| -> egui::Pos2 {
            egui::pos2(
                rect.center().x + pan.x + zoom * bx,
                rect.center().y + pan.y + zoom * by,
            )
        };
        let s = |v: f32| -> f32 { zoom * v }; // scale a length

        // ── Animation ──
        let t      = self.base.time_offset;
        let x_phys = self.x_anim(t); // physical [m], no clamp

        // Physical → visual pixels: normalise so x_ref shows as 0.20*base_scale travel
        let x_ref = (self.x0.abs()
            .max(self.force_amp / self.stiffness.max(1e-9))
            .max(0.001)) as f32;
        let px_per_m = base_scale * 0.20 / x_ref; // px/m in BASE (pre-zoom) space
        let x_base = if x_phys.is_finite() { (x_phys as f32 * px_per_m).clamp(-1e5, 1e5) } else { 0.0 };

        let f_now = if self.force_amp > 1e-6 {
            self.force_amp * (self.excit_freq * t).cos()
        } else { 0.0 };

        // ── Fixed base positions (base-space, centered at 0,0) ──
        let wall_bx  = -base_scale * 0.65;
        let mass_eq  =  base_scale * 0.15; // equilibrium attachment point
        let mass_bx  = mass_eq + x_base;   // current attachment (with displacement)

        // ── Wall ──
        let (wall_top_by, wall_bot_by) = (-base_scale * 0.42, base_scale * 0.42);
        painter.line_segment([p(wall_bx, wall_top_by), p(wall_bx, wall_bot_by)],
            egui::Stroke::new(3.0, text_c));
        for i in 0..10usize {
            let by = wall_top_by + (wall_bot_by - wall_top_by) * i as f32 / 9.0;
            painter.line_segment([p(wall_bx, by), p(wall_bx - s(9.0), by + s(9.0))],
                egui::Stroke::new(1.5, text_c.linear_multiply(0.7)));
        }

        // ── Spring (upper) ──
        let spring_by = -base_scale * 0.16;
        let n_coils = 8;
        let zigzag = base_scale * 0.065;
        let slen   = (mass_bx - wall_bx - 4.0 / zoom).max(base_scale * 0.08);
        let cw     = slen / (n_coils as f32 + 2.0);
        let mut spts: Vec<egui::Pos2> = Vec::new();
        spts.push(p(wall_bx, spring_by));
        spts.push(p(wall_bx + cw, spring_by));
        for i in 0..n_coils {
            let bx  = wall_bx + cw * (i as f32 + 1.5);
            let sign = if i%2==0 { 1.0 } else { -1.0 };
            spts.push(p(bx, spring_by + sign * zigzag));
        }
        spts.push(p(mass_bx - cw, spring_by));
        spts.push(p(mass_bx, spring_by));
        for w in spts.windows(2) {
            painter.line_segment([w[0], w[1]], egui::Stroke::new(2.0, theme.series_color(2)));
        }
        painter.text(p((wall_bx + mass_bx) * 0.5, spring_by - base_scale * 0.09),
            egui::Align2::CENTER_CENTER, "k",
            egui::FontId::proportional((14.0 * zoom.sqrt()).clamp(8.0, 22.0)), theme.series_color(2));

        // ── Damper (lower) ──
        let damp_by = base_scale * 0.16;
        let dmid_bx = (wall_bx + mass_bx) * 0.5;
        let bw = base_scale * 0.13;
        let bh = base_scale * 0.10;
        painter.line_segment([p(wall_bx, damp_by), p(dmid_bx - bw, damp_by)],
            egui::Stroke::new(2.0, theme.series_color(4)));
        let br = egui::Rect::from_center_size(p(dmid_bx, damp_by), egui::vec2(s(bw*2.0), s(bh)));
        painter.rect_stroke(br, 2.0, egui::Stroke::new(2.0, theme.series_color(4)));
        painter.line_segment([p(dmid_bx + bw*0.5, damp_by), p(mass_bx, damp_by)],
            egui::Stroke::new(2.5, theme.series_color(4)));
        painter.line_segment(
            [p(dmid_bx + bw*0.5, damp_by - bh*0.5), p(dmid_bx + bw*0.5, damp_by + bh*0.5)],
            egui::Stroke::new(3.0, theme.series_color(4)));
        painter.text(p(dmid_bx, damp_by + base_scale*0.10), egui::Align2::CENTER_CENTER, "c",
            egui::FontId::proportional((14.0 * zoom.sqrt()).clamp(8.0, 22.0)), theme.series_color(4));

        // Wall-side vertical connector
        painter.line_segment([p(wall_bx, spring_by), p(wall_bx, damp_by)],
            egui::Stroke::new(1.5, grid_c));

        // ── Mass block ──
        let mh = base_scale * 0.16;
        let mass_center_bx = mass_bx + mh;
        let mass_center_by = 0.0_f32;
        // Connectors to mass
        painter.line_segment([p(mass_bx, spring_by), p(mass_bx, mass_center_by)],
            egui::Stroke::new(1.5, grid_c));
        painter.line_segment([p(mass_bx, damp_by),  p(mass_bx, mass_center_by)],
            egui::Stroke::new(1.5, grid_c));
        let mrect = egui::Rect::from_center_size(
            p(mass_center_bx, mass_center_by), egui::vec2(s(mh*2.0), s(mh*2.2)));
        painter.rect_filled(mrect, 4.0, accent.linear_multiply(0.28));
        painter.rect_stroke(mrect, 4.0, egui::Stroke::new(2.5, accent));
        let fsize = (16.0 * zoom.sqrt()).clamp(8.0, 26.0);
        painter.text(mrect.center(), egui::Align2::CENTER_CENTER, "m",
            egui::FontId::proportional(fsize), accent);

        // ── Animated force vector ──
        if self.force_amp > 1e-6 {
            let fc = egui::Color32::from_rgb(220, 50, 30);
            let max_arr_b = base_scale * 0.28; // arrow max length in base space
            let frac = (f_now / self.force_amp).clamp(-1.0, 1.0) as f32;
            let arr_b = frac * max_arr_b;
            let start_bx = mass_center_bx + mh + 4.0 / zoom;
            let tip = p(start_bx + arr_b, mass_center_by);
            painter.line_segment([p(start_bx, mass_center_by), tip], egui::Stroke::new(2.5, fc));
            if arr_b.abs() * zoom > 6.0 {
                let d = if arr_b > 0.0 { 1.0f32 } else { -1.0 };
                painter.add(egui::Shape::convex_polygon(
                    vec![tip,
                         egui::pos2(tip.x - d*s(9.0), tip.y - s(5.5)),
                         egui::pos2(tip.x - d*s(9.0), tip.y + s(5.5))],
                    fc, egui::Stroke::NONE));
            }
            painter.text(p(start_bx + arr_b*0.5, mass_center_by - base_scale*0.08),
                egui::Align2::CENTER_CENTER,
                &format!("F={:.1} N", f_now),
                egui::FontId::proportional((11.0 * zoom.sqrt()).clamp(7.0, 18.0)), fc);
        }

        // ── Displacement indicator (always in screen-space at bottom of view) ──
        let ay   = rect.center().y + pan.y + s(base_scale * 0.40);
        let rx   = rect.center().x + pan.x + s(mass_eq); // equilibrium in screen coords
        let dc   = egui::Color32::from_rgb(50, 190, 50);
        painter.line_segment(
            [egui::pos2(rx, ay - 5.0), egui::pos2(rx, ay + 5.0)],
            egui::Stroke::new(1.5, grid_c));
        let x_px_screen = x_base * zoom;
        if x_px_screen.abs() > 2.0 {
            let tip = egui::pos2(rx + x_px_screen, ay);
            painter.line_segment([egui::pos2(rx, ay), tip], egui::Stroke::new(2.0, dc));
            let d = if x_px_screen > 0.0 { 1.0f32 } else { -1.0 };
            painter.add(egui::Shape::convex_polygon(
                vec![tip,
                     egui::pos2(tip.x - d*8.0, tip.y - 5.0),
                     egui::pos2(tip.x - d*8.0, tip.y + 5.0)],
                dc, egui::Stroke::NONE));
        }
        painter.text(
            egui::pos2(rx + x_px_screen*0.5, ay + 14.0),
            egui::Align2::CENTER_CENTER,
            &format!("x = {:.4} m", x_phys),
            egui::FontId::proportional(12.0), dc);

        // ── Info bar (always fixed at bottom of rect, independent of pan/zoom) ──
        let wn = self.wn(); let zeta = self.zeta(); let r = self.r();
        painter.text(egui::pos2(rect.center().x, rect.bottom() - 11.0),
            egui::Align2::CENTER_CENTER,
            &format!("omega_n={:.2} rad/s ({:.2} Hz)  zeta={:.3}  {}",
                wn, wn/(2.0*PI), zeta, detect_case(zeta, self.force_amp, r)),
            egui::FontId::proportional(11.0), egui::Color32::from_gray(150));

        // ── Zoom hint (top-right, fixed) ──
        painter.text(
            egui::pos2(rect.right() - 6.0, rect.top() + 8.0),
            egui::Align2::RIGHT_TOP,
            &format!("zoom {:.1}x  scroll=zoom  drag=pan  dbl=reset", zoom),
            egui::FontId::proportional(9.5),
            egui::Color32::from_gray(130));
    }

    // ── LaTeX equations ────────────────────────────────────────────────────
    #[cfg(feature = "mathjax")]
    fn draw_equations(&mut self, ui: &mut egui::Ui) {
        let dark  = self.base.theme.dark_mode;
        let scale = self.render_scale;
        let ds    = self.display_scale;
        let acc   = self.base.theme.accent_color();
        let tc    = self.base.theme.text_color();

        let (m, c, k, f0, omega) = (self.mass, self.damping, self.stiffness,
                                     self.force_amp, self.excit_freq);
        let wn   = self.wn();
        let zeta = self.zeta();
        let r    = self.r();
        let h    = amplification(r, zeta);
        let phi  = phase_lag(r, zeta).to_degrees();

        ui.vertical(|ui| {
            ui.label(egui::RichText::new("SDOF Theory").strong().size(15.0).color(acc));
            ui.add_space(4.0);

            let ctx = ui.ctx().clone();

            // Helper: render one combined equation image
            let show1 = |ui: &mut egui::Ui, cache: &mut LatexCache,
                          lbl: &str, key: &str, latex: &str| {
                ui.label(egui::RichText::new(lbl).size(10.0).color(tc.linear_multiply(0.5)));
                if let Some(e) = cache.get_or_render(key, &ctx, latex, scale, dark) {
                    ui.add(egui::Image::new(egui::load::SizedTexture::new(
                        e.texture.id(), egui::vec2(e.width*ds, e.height*ds))));
                } else { ui.label("..."); }
                ui.add_space(2.0);
            };

            // EOM — symbolic form, then substituted form
            show1(ui, &mut self.cache, "Equation of Motion",
                "eom_sym",
                r"m\ddot{x}+c\dot{x}+kx=F_0\cos(\Omega t)");
            show1(ui, &mut self.cache, "",
                &format!("eom_num_{}", hash_f64s(&[m,c,k,f0])),
                &format!(r"{}\ddot{{x}}+{}\dot{{x}}+{}x={}\cos(\Omega t)",
                    cv(m,1), cv(c,1), cv(k,0), cv(f0,1)));

            ui.separator();

            // omega_n
            show1(ui, &mut self.cache, "Natural Frequency",
                &format!("wn_{}", hash_f64s(&[k,m])),
                &format!(r"\omega_n=\sqrt{{\dfrac{{k}}{{m}}}}={}\;\mathrm{{rad/s}}",
                    cv(wn,2)));

            ui.separator();

            // zeta
            show1(ui, &mut self.cache, "Damping Ratio",
                &format!("z_{}", hash_f64s(&[m,k,c])),
                &format!(r"\zeta=\dfrac{{c}}{{2\sqrt{{mk}}}}={}",
                    cv(zeta,3)));

            if f0 > 1e-6 {
                ui.separator();

                // r
                show1(ui, &mut self.cache, "Frequency Ratio",
                    &format!("r_{}", hash_f64s(&[omega,wn])),
                    &format!(r"r=\dfrac{{\Omega}}{{\omega_n}}={}",
                        cv(r,3)));

                ui.separator();

                // H(r)
                let h_latex = if detect_case(zeta, f0, r) == "Resonance" {
                    format!(r"H(r)\to\infty\;\text{{(resonance)}}")
                } else {
                    format!(r"H(r)=\dfrac{{1}}{{\sqrt{{(1-r^2)^2+(2\zeta r)^2}}}}={}",
                        cv(h,2))
                };
                show1(ui, &mut self.cache, "Amplification",
                    &format!("h_{}", hash_f64s(&[r,zeta])), &h_latex);

                ui.separator();

                // Phase
                show1(ui, &mut self.cache, "Phase Lag",
                    &format!("phi_{}", hash_f64s(&[r,zeta])),
                    &format!(r"\varphi=\arctan\!\left(\dfrac{{2\zeta r}}{{1-r^2}}\right)={}^\circ",
                        cv(phi,1)));
            }

            ui.separator();
            let fn_hz = wn/(2.0*PI);
            let fe_hz = omega/(2.0*PI);
            let case   = detect_case(zeta, f0, r);
            ui.label(egui::RichText::new(
                format!("fn={:.2} Hz   fe={:.2} Hz   r={:.3}",
                    fn_hz, fe_hz, r))
                .size(11.0).color(tc.linear_multiply(0.65)));
            ui.label(egui::RichText::new(format!("> {}", case))
                .strong().size(12.0).color(case_color(case)));

            // ── Preset note (bottom) ──
            if !self.preset_note.is_empty() {
                let fn_hz2 = wn / (2.0 * PI);

                // Each parameter is a separate small LaTeX render so that
                // `ui.horizontal_wrapped` can reflow them when the panel is narrow.
                let mut param_defs: Vec<(String, String)> = vec![
                    (format!("np_wn_{}", hash_f64s(&[wn])),
                     format!(r"\omega_n={}\,\mathrm{{rad/s}}", cv(wn, 2))),
                    (format!("np_fn_{}", hash_f64s(&[wn])),
                     format!(r"f_n={}\,\mathrm{{Hz}}", cv(fn_hz2, 3))),
                    (format!("np_z_{}", hash_f64s(&[zeta])),
                     format!(r"\zeta={}", cv(zeta, 4))),
                ];
                if f0 > 1e-6 {
                    param_defs.push((format!("np_r_{}", hash_f64s(&[r])),
                                     format!(r"r={}", cv(r, 3))));
                    param_defs.push((format!("np_h_{}", hash_f64s(&[r, zeta])),
                                     format!(r"H(r)={}", cv(h, 2))));
                }

                // Pre-render each param and collect (TextureId, display size).
                // Must be done before the Frame closure to satisfy the borrow checker.
                let tex_infos: Vec<Option<(egui::TextureId, egui::Vec2)>> = param_defs
                    .iter()
                    .map(|(key, latex)| {
                        self.cache
                            .get_or_render(key, &ctx, latex, scale, dark)
                            .map(|e| (e.texture.id(), egui::vec2(e.width * ds, e.height * ds)))
                    })
                    .collect();

                egui::Frame::none()
                    .fill(if self.base.theme.dark_mode {
                        egui::Color32::from_rgb(28, 38, 54)
                    } else {
                        egui::Color32::from_rgb(225, 235, 248)
                    })
                    .inner_margin(egui::Margin::same(8.0))
                    .outer_margin(egui::Margin { left: 0.0, right: 12.0, top: 8.0, bottom: 4.0 })
                    .rounding(egui::Rounding::same(5.0))
                    .show(ui, |ui| {
                        // Param chips — wrap to next line when panel is narrow
                        ui.horizontal_wrapped(|ui| {
                            for info in &tex_infos {
                                if let Some((id, size)) = info {
                                    ui.add(egui::Image::new(
                                        egui::load::SizedTexture::new(*id, *size)));
                                    ui.add_space(6.0);
                                }
                            }
                        });
                        ui.add_space(4.0);
                        // Plain-text description, scales with display_scale
                        ui.label(egui::RichText::new(self.preset_note)
                            .size(11.0 * ds)
                            .color(self.base.theme.text_color()));
                    });
            }
        });
    }

    // ── Plain-text equations (no MathJax) — used in the WebAssembly build ──
    #[cfg(not(feature = "mathjax"))]
    fn draw_equations_plain(&mut self, ui: &mut egui::Ui) {
        let acc = self.base.theme.accent_color();
        let tc  = self.base.theme.text_color();

        let (m, c, k, f0, omega) = (self.mass, self.damping, self.stiffness,
                                     self.force_amp, self.excit_freq);
        let wn   = self.wn();
        let zeta = self.zeta();
        let r    = self.r();
        let h    = amplification(r, zeta);
        let phi  = phase_lag(r, zeta).to_degrees();
        let case = detect_case(zeta, f0, r);

        // Helper: small caption above each equation block
        let caption = |ui: &mut egui::Ui, text: &str, tc: egui::Color32| {
            ui.label(egui::RichText::new(text).size(10.0).color(tc.linear_multiply(0.55)));
        };
        // Helper: render an equation line in monospace
        let eq = |ui: &mut egui::Ui, text: String| {
            ui.label(egui::RichText::new(text).monospace().size(13.0));
        };

        ui.vertical(|ui| {
            ui.label(egui::RichText::new("SDOF Theory").strong().size(15.0).color(acc));
            ui.add_space(4.0);

            caption(ui, "Equation of Motion", tc);
            eq(ui, "m·x'' + c·x' + k·x = F0·cos(Ω t)".to_string());
            eq(ui, format!("{:.1}·x'' + {:.1}·x' + {:.0}·x = {:.1}·cos(Ω t)", m, c, k, f0));
            ui.separator();

            caption(ui, "Natural Frequency", tc);
            eq(ui, format!("ω_n = √(k/m) = {:.2} rad/s", wn));
            ui.separator();

            caption(ui, "Damping Ratio", tc);
            eq(ui, format!("ζ = c / (2·√(m·k)) = {:.3}", zeta));

            if f0 > 1e-6 {
                ui.separator();
                caption(ui, "Frequency Ratio", tc);
                eq(ui, format!("r = Ω / ω_n = {:.3}", r));

                ui.separator();
                caption(ui, "Amplification", tc);
                if case == "Resonance" {
                    eq(ui, "H(r) → ∞   (resonance)".to_string());
                } else {
                    eq(ui, format!("H(r) = 1 / √((1 - r²)² + (2·ζ·r)²) = {:.2}", h));
                }

                ui.separator();
                caption(ui, "Phase Lag", tc);
                eq(ui, format!("φ = arctan( 2·ζ·r / (1 - r²) ) = {:.1}°", phi));
            }

            ui.separator();
            let fn_hz = wn / (2.0 * PI);
            let fe_hz = omega / (2.0 * PI);
            ui.label(egui::RichText::new(
                format!("fn = {:.2} Hz    fe = {:.2} Hz    r = {:.3}", fn_hz, fe_hz, r))
                .size(11.0).color(tc.linear_multiply(0.65)));
            ui.label(egui::RichText::new(format!("> {}", case))
                .strong().size(12.0).color(case_color(case)));

            // ── Preset note (bottom) ──
            if !self.preset_note.is_empty() {
                egui::Frame::none()
                    .fill(if self.base.theme.dark_mode {
                        egui::Color32::from_rgb(28, 38, 54)
                    } else {
                        egui::Color32::from_rgb(225, 235, 248)
                    })
                    .inner_margin(egui::Margin::same(8.0))
                    .outer_margin(egui::Margin { left: 0.0, right: 12.0, top: 8.0, bottom: 4.0 })
                    .rounding(egui::Rounding::same(5.0))
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(
                            format!("ω_n = {:.2} rad/s    f_n = {:.3} Hz    ζ = {:.4}",
                                    wn, fn_hz, zeta))
                            .monospace().size(11.0));
                        if f0 > 1e-6 {
                            ui.label(egui::RichText::new(
                                format!("r = {:.3}    H(r) = {:.2}", r, h))
                                .monospace().size(11.0));
                        }
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new(self.preset_note)
                            .size(11.0)
                            .color(self.base.theme.text_color()));
                    });
            }
        });
    }

    // ── Time response plot ─────────────────────────────────────────────────
    fn draw_time_plot(&mut self, ui: &mut egui::Ui) {
        // Use the cached response (same data as animation)
        let (xpts, vpts, fpts, epp, epn, t_cur) = {
            let resp  = self.resp_cache.as_ref().unwrap();
            let theme = &self.base.theme;
            let zeta  = self.zeta();
            let wn    = self.wn();
            let xp: PlotPoints = resp.t.iter().zip(resp.x.iter()).map(|(&t,&x)| [t,x]).collect();
            let vp: PlotPoints = resp.t.iter().zip(resp.v.iter()).map(|(&t,&v)| [t,v]).collect();

            // Force F(t) = F0·cos(Ωt), normalized to peak |x| so phase shift is visible at any scale
            let fp = if self.show_force && self.force_amp > 1e-6 {
                let x_max = resp.x.iter().cloned().fold(0.0_f64, |a, x| a.max(x.abs()));
                // fall back to static deflection if x is near zero (isolation regime)
                let scale = if x_max > 1e-12 { x_max } else { self.force_amp / self.stiffness };
                let omega = self.excit_freq;
                Some(resp.t.iter().map(|&t| [t, scale * (omega * t).cos()]).collect::<PlotPoints>())
            } else { None };

            // Decay envelope
            let (ep, en) = if self.show_envelope && self.force_amp < 1e-6
                              && zeta > 1e-6 && zeta < 1.0
            {
                let wd = self.wd().max(1e-9);
                let a0 = (self.x0*self.x0 + ((self.v0+zeta*wn*self.x0)/wd).powi(2)).sqrt();
                let e_col = theme.series_color(3).linear_multiply(0.5);
                let ep: PlotPoints = resp.t.iter().map(|&t| [t, a0*(-zeta*wn*t).exp()]).collect();
                let en: PlotPoints = resp.t.iter().map(|&t| [t,-a0*(-zeta*wn*t).exp()]).collect();
                (Some((ep, e_col)), Some(en))
            } else { (None, None) };

            let tc = self.base.time_offset % self.t_end.max(1e-3);
            (xp, vp, fp, ep, en, tc)
        };

        let theme  = &self.base.theme;
        let accent = theme.accent_color();
        let zeta   = self.zeta();
        let case   = detect_case(zeta, self.force_amp, self.r());

        Plot::new("sdof_time")
            .height(ui.available_height() - 4.0)
            .x_axis_label("t [s]")
            .y_axis_label("x [m]")
            .auto_bounds(egui::Vec2b::new(false, true))
            .include_x(0.0)
            .include_x(self.t_end)
            .legend(egui_plot::Legend::default())
            .show(ui, |pu: &mut egui_plot::PlotUi| {
                pu.line(Line::new(xpts).name(format!("x(t) - {}", case)).color(accent).width(2.0));
                if self.show_velocity {
                    pu.line(Line::new(vpts).name("v(t) [m/s]").color(theme.series_color(1)).width(1.5));
                }
                if let Some(fp) = fpts {
                    pu.line(Line::new(fp).name("F(t) [scaled]").color(theme.series_color(5)).width(1.5).style(egui_plot::LineStyle::dashed_loose()));
                }
                if let (Some((ep, ec)), Some(en)) = (epp, epn) {
                    pu.line(Line::new(ep).name("Envelope").color(ec).width(1.0));
                    pu.line(Line::new(en).color(ec).width(1.0));
                }
                pu.vline(VLine::new(t_cur).color(egui::Color32::RED).width(1.5));
            });
    }

    // ── FRF plots (H and phi side by side) ────────────────────────────────
    fn draw_frf_plot(&mut self, ui: &mut egui::Ui) {
        let total_h = ui.available_height();
        let avail_w = ui.available_width();
        let half_w  = (avail_w - 8.0) / 2.0;
        let plot_h  = (total_h - 4.0).max(60.0);

        let zeta    = self.zeta();
        let r_cur   = self.r();
        let h_cur   = amplification(r_cur, zeta);
        let phi_cur = phase_lag(r_cur, zeta).to_degrees();
        let theme   = &self.base.theme;
        let accent  = theme.accent_color();

        let hc   = compute_frf_curve(zeta);
        let phic = compute_phase_curve(zeta);

        ui.horizontal(|ui| {
            // ── H(r) ──
            ui.allocate_ui_with_layout(
                egui::vec2(half_w, plot_h),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    Plot::new("sdof_h")
                        .height(plot_h)
                        .x_axis_label("r = Omega/omega_n")
                        .y_axis_label("H [-]")
                        .auto_bounds(egui::Vec2b::new(false, false))
                        .include_x(0.0).include_x(3.0)
                        .include_y(0.0).include_y(10.0)
                        .legend(egui_plot::Legend::default())
                        .show(ui, |pu: &mut egui_plot::PlotUi| {
                            for (i, &zr) in ZETA_REFS.iter().enumerate() {
                                let pts: PlotPoints = compute_frf_curve(zr).into_iter().collect();
                                pu.line(Line::new(pts).name(format!("z={}", zr))
                                    .color(zeta_color(i)).width(1.2));
                            }
                            pu.line(Line::new(Into::<PlotPoints>::into(hc))
                                .name(format!("z={}", fmt_trim(zeta))).color(accent).width(2.5));
                            pu.points(Points::new(vec![[r_cur, h_cur]])
                                .name(format!("H={:.2}", h_cur))
                                .color(egui::Color32::RED).radius(6.0));
                            pu.vline(VLine::new(1.0).color(theme.grid_color()).width(1.0));
                            pu.vline(VLine::new(r_cur)
                                .color(egui::Color32::RED.linear_multiply(0.35)).width(1.0));
                        });
                });

            ui.separator();

            // ── phi(r) ──
            ui.allocate_ui_with_layout(
                egui::vec2(half_w, plot_h),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    Plot::new("sdof_phi")
                        .height(plot_h)
                        .x_axis_label("r = Omega/omega_n")
                        .y_axis_label("phi [deg]")
                        .auto_bounds(egui::Vec2b::new(false, true))
                        .include_x(0.0).include_x(3.0).include_y(0.0).include_y(180.0)
                        .legend(egui_plot::Legend::default())
                        .show(ui, |pu: &mut egui_plot::PlotUi| {
                            for (i, &zr) in ZETA_REFS.iter().enumerate() {
                                let pts: PlotPoints = compute_phase_curve(zr).into_iter().collect();
                                pu.line(Line::new(pts).name(format!("z={}", zr))
                                    .color(zeta_color(i)).width(1.2));
                            }
                            pu.line(Line::new(Into::<PlotPoints>::into(phic))
                                .name(format!("z={}", fmt_trim(zeta)))
                                .color(theme.series_color(2)).width(2.5));
                            pu.points(Points::new(vec![[r_cur, phi_cur]])
                                .name(format!("phi={:.1}deg", phi_cur))
                                .color(egui::Color32::RED).radius(6.0));
                            pu.vline(VLine::new(1.0).color(theme.grid_color()).width(1.0));
                            pu.vline(VLine::new(r_cur)
                                .color(egui::Color32::RED.linear_multiply(0.35)).width(1.0));
                            pu.hline(egui_plot::HLine::new(90.0)
                                .color(theme.grid_color()).width(1.0));
                        });
                });
        });
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// ── IwesApp ──
// ══════════════════════════════════════════════════════════════════════════════

impl IwesApp for SdofApp {
    fn title(&self) -> &str { "SDOF Dynamics Explorer - IWES" }
    fn base(&self)         -> &AppBase      { &self.base }
    fn base_mut(&mut self) -> &mut AppBase  { &mut self.base }

    fn controls(&mut self, ui: &mut egui::Ui) {
        if self.base.theme.dark_mode != self.last_dark {
            self.cache.invalidate();
            self.last_dark = self.base.theme.dark_mode;
        }

        // ── Presets ──
        section_header(ui, "Presets");
        ui.horizontal_wrapped(|ui| {
            for (name, m, k, c, f0, omega, x0, v0, t_end, note) in [
                // ── Generic dynamics ──
                ("Free undamped",  1.0, 100.0,  0.0,  0.0,  0.0, 0.10, 0.0, 20.0,
                    "Ideal SHO (simple harmonic oscillator). No energy dissipation — \
                     the mass oscillates forever at the natural frequency omega_n. \
                     Basis for all vibration analysis."),
                ("Underdamped",    1.0, 100.0,  2.0,  0.0,  0.0, 0.10, 0.0, 20.0,
                    "Damping ratio zeta < 1. The system oscillates with an amplitude \
                     that decays exponentially as exp(-zeta*omega_n*t). \
                     Most engineering structures fall in this regime (zeta << 1)."),
                ("Critical",       1.0, 100.0, 20.0,  0.0,  0.0, 0.10, 0.0, 10.0,
                    "Critically damped: zeta = 1. Returns to equilibrium as fast as \
                     possible without oscillating. Transition between under- and \
                     overdamped behaviour. Used in some shock absorber designs."),
                ("Overdamped",     1.0, 100.0, 40.0,  0.0,  0.0, 0.10, 0.0, 10.0,
                    "Overdamped: zeta > 1. Pure exponential decay, no oscillation. \
                     Slower than critically damped. Occurs in heavily damped systems \
                     such as door closers or some hydraulic mounts."),
                ("Forced r=0.5",   1.0, 100.0,  2.0, 10.0,  5.0, 0.00, 0.0, 40.0,
                    "Forced vibration below resonance (r = Omega/omega_n = 0.5). \
                     The response is roughly in phase with the force. \
                     Amplitude amplification H ≈ 1.3 (moderate)."),
                ("Resonance",      1.0, 100.0,  0.5, 10.0, 10.0, 0.00, 0.0, 60.0,
                    "Excitation at the natural frequency (r ≈ 1) with very low damping \
                     (zeta ≈ 0.025). Amplitude grows without bound for zeta = 0; \
                     real structures limit growth via nonlinearity or failure."),
                ("Forced r=2",     1.0, 100.0,  2.0, 10.0, 20.0, 0.00, 0.0, 30.0,
                    "Forced vibration well above resonance (r = 2). \
                     The response is 180 deg out of phase with the force and \
                     the amplitude is attenuated (H < 1). This is the isolation region."),
                ("Car suspension", 300.0, 15000.0, 900.0, 0.0, 0.0, 0.05, 0.0, 15.0,
                    "Quarter-car model. A typical compact car: m = 300 kg (sprung mass), \
                     k = 15 kN/m (spring), c = 900 Ns/m (damper). \
                     zeta ≈ 0.55 — slightly overdamped for ride comfort."),
                // ── Wind energy ──
                ("Tower FA (free)", 3e5, 1.07e6, 1.13e4, 0.0, 0.0, 0.10, 0.0, 80.0,
                    "FA = Fore-Aft (along-wind) bending of the tower. \
                     Represents a 5 MW onshore turbine tower (height ≈ 100 m, \
                     top mass ≈ 300 t). Natural frequency fn ≈ 0.30 Hz, \
                     structural damping zeta ≈ 1.1 %. \
                     Free vibration from a 0.1 m tip displacement."),
                ("Tower FA (1P)",   3e5, 1.07e6, 1.13e4, 5.0e4, 1.26, 0.00, 0.0, 120.0,
                    "1P = Once-Per-Revolution rotor excitation. \
                     At 12 rpm the rotor turns at 0.2 Hz (1.26 rad/s), giving r ≈ 0.67. \
                     Mass imbalance or aerodynamic asymmetry produce a 1P force on the tower. \
                     Operating below resonance ('soft-stiff' design) is typical for onshore turbines."),
                ("Tower 3P resonance", 3e5, 4.27e6, 5.0e3, 5.0e4, 3.77, 0.00, 0.0, 200.0,
                    "3P = Three-Per-Revolution blade-passing excitation (3 blades * 12 rpm = \
                     0.6 Hz, 3.77 rad/s). This preset places the tower natural frequency \
                     exactly at 3P — a resonance condition to be strictly avoided in design. \
                     Low damping (zeta ≈ 0.4 %) leads to large, growing oscillations."),
                ("Blade flapwise",  1.7e4, 6.72e5, 2.14e3, 0.0, 0.0, 0.50, 0.0, 20.0,
                    "Flapwise = out-of-plane blade bending (in the direction of wind thrust). \
                     Represents the 1st flapwise mode of a ~61 m blade (5 MW class). \
                     fn ≈ 1.0 Hz. Aerodynamic damping adds to structural damping, \
                     giving effective zeta ≈ 1.4 %. Free vibration from 0.5 m tip deflection."),
                ("Blade edgewise",  1.7e4, 1.73e6, 930.0, 0.0, 0.0, 0.10, 0.0, 15.0,
                    "Edgewise = in-plane blade bending (gravity + drag direction). \
                     Stiffer than flapwise: fn ≈ 1.6 Hz. \
                     Critically: aerodynamic forces provide negative damping in edgewise, \
                     so total damping is very low (zeta ≈ 0.5 %). \
                     Edgewise instability (flutter) is a key design driver for large blades."),
                ("Drivetrain",      1e5, 1.58e7, 4.0e4, 0.0, 0.0, 0.01, 0.0, 10.0,
                    "First torsional mode of the drivetrain: rotor — main shaft — gearbox \
                     — high-speed shaft — generator. Equivalent linear SDOF with \
                     fn ≈ 2 Hz, zeta ≈ 1.6 %. \
                     Drivetrain resonance from grid transients or emergency stops \
                     can cause high torque peaks and fatigue damage."),
                ("Monopile (OWT)",  8e5, 1.97e6, 4.0e4, 0.0, 0.0, 0.05, 0.0, 100.0,
                    "OWT = Offshore Wind Turbine on a monopile (steel tube driven into seabed). \
                     Larger structural mass (rotor+nacelle+tower+pile ≈ 800 t) and \
                     higher damping (soil + hydrodynamic damping, zeta ≈ 2 %) compared \
                     to onshore. Target fn ≈ 0.25 Hz, placed in the '1P–3P gap' to \
                     avoid rotor excitation. Free vibration from 0.05 m tip displacement."),
                ("Monopile+waves",  8e5, 1.97e6, 4.0e4, 2.0e5, 0.628, 0.00, 0.0, 120.0,
                    "Monopile forced by first-order wave loads. \
                     Wave period ≈ 10 s (f = 0.1 Hz, Omega = 0.628 rad/s), \
                     well below the tower natural frequency (r ≈ 0.40). \
                     The structure is in the 'quasi-static' wave regime — \
                     the dynamic amplification H ≈ 1.2 is moderate but important for fatigue."),
            ] {
                if ui.small_button(name).clicked() {
                    self.mass = m; self.stiffness = k; self.damping = c;
                    self.force_amp = f0; self.excit_freq = omega;
                    self.x0 = x0; self.v0 = v0; self.t_end = t_end;
                    self.base.t_max = t_end;
                    self.base.time_offset = 0.0;
                    self.resp_cache = None;
                    self.cache.invalidate();
                    self.preset_note = note;
                }
            }
        });

        section_header(ui, "System Parameters");
        labeled_slider_log(ui, "Mass m:",      &mut self.mass,      0.1..=1.0e6, " kg");
        labeled_slider_log(ui, "Stiffness k:", &mut self.stiffness, 1.0..=1.0e8, " N/m");
        labeled_slider_log(ui, "Damping c:",   &mut self.damping,   0.0..=1.0e6, " Ns/m");

        let wn   = self.wn();
        let zeta = self.zeta();
        let r    = self.r();
        ui.add_space(3.0);
        ui.label(egui::RichText::new(
            format!("omega_n = {:.2} rad/s  ({:.3} Hz)", wn, wn/(2.0*PI)))
            .size(12.0).color(self.base.theme.accent_color()));
        ui.label(egui::RichText::new(
            format!("zeta = {:.4}    wd = {:.2} rad/s", zeta, self.wd()))
            .size(12.0).color(self.base.theme.accent_color()));

        let case = detect_case(zeta, self.force_amp, r);
        ui.add_space(3.0);
        ui.label(egui::RichText::new(format!("> {}", case))
            .strong().size(13.0).color(case_color(case)));

        section_header(ui, "Excitation  (F0 = 0 -> free)");
        labeled_slider_log(ui, "Force F0:", &mut self.force_amp, 0.0..=1.0e6, " N");
        ui.horizontal(|ui| {
            ui.label("Omega [rad/s]:");
            ui.add(egui::Slider::new(&mut self.excit_freq, 0.0..=200.0)
                .suffix(" rad/s").step_by(0.01));
            if ui.small_button("→ res.").on_hover_text("Set Ω = ωₙ (move red point to resonance peak)").clicked() {
                self.excit_freq = wn.min(200.0);
            }
        });
        // r slider: directly positions the red operating point on the FRF plots
        let mut r_edit = r;
        let r_changed = ui.horizontal(|ui| {
            ui.label("r = Ω/ωₙ:");
            ui.add(egui::Slider::new(&mut r_edit, 0.0..=3.0).step_by(0.001))
                .on_hover_text("Drag to move the red point along the FRF curves.\nCtrl+drag for fine steps.")
                .changed()
        }).inner;
        if r_changed && wn > 1e-9 {
            self.excit_freq = (r_edit * wn).clamp(0.0, 200.0);
        }
        ui.label(egui::RichText::new(
            format!("r={:.3}  Ω={:.3} rad/s  ωₙ={:.3} rad/s", r, self.excit_freq, wn))
            .size(11.0).color(self.base.theme.text_color().linear_multiply(0.7)));

        section_header(ui, "Initial Conditions");
        labeled_slider_step(ui, "x0:", &mut self.x0, -5.0..=5.0,   " m",   0.005);
        labeled_slider_step(ui, "v0:", &mut self.v0, -50.0..=50.0,  " m/s", 0.05);

        section_header(ui, "Simulation");
        let old_tend = self.t_end;
        labeled_slider_step(ui, "t_end:", &mut self.t_end, 1.0..=500.0, " s", 0.5);
        if (self.t_end - old_tend).abs() > 1e-9 {
            self.base.t_max = self.t_end;
            self.resp_cache = None; // force recompute
        }
        ui.checkbox(&mut self.show_velocity, "Show velocity");
        ui.checkbox(&mut self.show_envelope, "Show decay envelope");
        ui.checkbox(&mut self.show_force,    "Show force F(t)");

        section_header(ui, "Equation Rendering");
        ui.add(egui::Slider::new(&mut self.render_scale,  2.0..=8.0).text("Quality").step_by(0.5));
        ui.add(egui::Slider::new(&mut self.display_scale, 0.5..=2.0).text("Scale").step_by(0.05));
        if ui.button("Clear cache").clicked() { self.cache.invalidate(); }
    }

    fn content(&mut self, ui: &mut egui::Ui) {
        self.ensure_resp();

        let total_h  = ui.available_height();
        let avail_w  = ui.available_width();
        // Guard against narrow viewports (mobile / split screens) so that
        // clamp(min, max) never has min > max.
        let lw_max   = (avail_w - 220.0).max(320.0);
        let left_w   = (avail_w * self.base.content_split).clamp(320.0, lw_max);
        let bar_h   = 30.0_f32;
        let hnd_h   =  4.0_f32; // drag handle height
        // Total height available for the three resizable sections
        let avail_h = (total_h - bar_h - hnd_h * 2.0 - 2.0).max(300.0);
        let draw_h  = (avail_h * self.draw_h_frac).clamp(60.0, avail_h - 140.0);
        let remain  = avail_h - draw_h;
        let time_h  = (remain * self.time_h_frac).clamp(60.0, remain - 60.0);
        let frf_h   = (remain - time_h).max(60.0);

        ui.horizontal(|ui| {
            // ── Left column: drawing + transport + time plot + FRF ──
            ui.allocate_ui_with_layout(
                egui::vec2(left_w, total_h),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    // Schematic
                    ui.allocate_ui_with_layout(
                        egui::vec2(left_w, draw_h),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| { self.draw_schematic(ui); });

                    // Drag handle: drawing ↔ transport
                    let d1 = hsplit_handle(ui, left_w);
                    if d1 != 0.0 {
                        self.draw_h_frac = ((draw_h + d1) / avail_h).clamp(0.10, 0.75);
                    }

                    // Transport bar
                    ui.allocate_ui_with_layout(
                        egui::vec2(left_w, bar_h),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            let lbl = if self.base.animate { "Pause" } else { "Run" };
                            if ui.button(lbl).clicked() { self.base.animate = !self.base.animate; }
                            if ui.button("Reset").clicked() { self.base.time_offset = 0.0; }
                            ui.add_space(6.0);
                            let t_max = self.t_end.max(1e-3);
                            ui.style_mut().spacing.slider_width = (left_w - 160.0).max(60.0);
                            ui.add(egui::Slider::new(&mut self.base.time_offset, 0.0..=t_max)
                                .show_value(false));
                            ui.label(format!("{:.2} s", self.base.time_offset));
                        });

                    // Time plot
                    ui.allocate_ui_with_layout(
                        egui::vec2(left_w, time_h),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| { self.draw_time_plot(ui); });

                    // Drag handle: time ↔ FRF
                    let d2 = hsplit_handle(ui, left_w);
                    if d2 != 0.0 {
                        self.time_h_frac = ((time_h + d2) / remain).clamp(0.15, 0.85);
                    }

                    // FRF plots
                    ui.allocate_ui_with_layout(
                        egui::vec2(left_w, frf_h),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| { self.draw_frf_plot(ui); });
                });

            // ── Draggable split handle ──
            let delta = vsplit_handle(ui, total_h);
            if delta != 0.0 {
                self.base.content_split =
                    ((left_w + delta) / avail_w).clamp(0.25, 0.80);
            }

            // ── Right column: theory equations (full height) ──
            #[cfg(feature = "mathjax")]
            {
                let right_w = (avail_w - left_w - 6.0).max(180.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(right_w, total_h),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(total_h)
                            .show(ui, |ui| {
                                self.draw_equations(ui);
                            });
                    });
            }
            // Plain-text equations fallback when MathJax isn't available
            // (e.g. WebAssembly build — V8/MathJax can't be compiled to WASM).
            #[cfg(not(feature = "mathjax"))]
            {
                let right_w = (avail_w - left_w - 6.0).max(180.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(right_w, total_h),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(total_h)
                            .show(ui, |ui| {
                                self.draw_equations_plain(ui);
                            });
                    });
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    run_app::<SdofApp>("SDOF Dynamics Explorer - IWES", [1400.0, 900.0])
}
