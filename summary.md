# SDOF Dynamics Explorer — Project Summary

**Author:** Basem Rajjoub, Institut für Windenergiesysteme (IWES), Leibniz Universität Hannover  
**License:** MIT  
**Stack:** Rust + eframe 0.29.1 / egui 0.29, compiled to native (desktop) and WebAssembly (browser)

---

## Purpose

Interactive educational tool for single-degree-of-freedom (mass-spring-damper) dynamics.  
Equation of motion: `m·x'' + c·x' + k·x = F₀·cos(Ω·t)`

The app visualises:
- Animated mechanical schematic (spring, damper, mass block, force arrow)
- Time-domain response x(t) via RK4 integration
- Frequency Response Function H(r) and phase lag curves
- Theory equations (LaTeX via MathJax on native; plain Unicode on WASM)

Wind-turbine presets are built in (tower 1P/3P resonance, blade flapwise/edgewise, drivetrain, monopile OWT, monopile+waves).

---

## Repository Layout

```
sdof_dynamics/
├── apps/
│   └── sdof_dynamics.rs        # The app (1230 lines) — both native and WASM binary
├── shared/
│   ├── template.rs             # IWES app framework (734 lines) — DO NOT modify casually
│   ├── viewport3d.rs           # Blender-style 3D camera controls (626 lines)
│   └── figures/
│       ├── 01-LUH-Logo.png     # Embedded in binary via include_bytes!
│       └── 02-IWES-Logo.png
├── web/
│   └── index.html              # Web entry point (served with WASM build)
├── build/
│   ├── build_web.sh            # Linux/Mac WASM build script
│   ├── BUILD_WEB.bat           # Windows WASM build script
│   └── WEB_BUILD_INSTRUCTIONS.md
├── tools/
│   └── wasm-bindgen-0.2.114-x86_64-pc-windows-msvc/  # Prebuilt bindgen (Windows)
├── _site/                      # Generated — WASM build output (gitignored)
├── Cargo.toml
├── Cargo.lock
└── .github/workflows/pages.yml # CI: build WASM → deploy to GitHub Pages
```

---

## Cargo.toml — Key Facts

```toml
[dependencies]
eframe       = { version = "0.29", features = ["wgpu"] }
egui_plot    = "0.29"
image        = { version = "0.25", default-features = false, features = ["png"] }
web-sys      = "0.3"
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"

[target.'cfg(target_arch = "wasm32")'.dependencies]
console_error_panic_hook = "0.1"   # prints Rust panics to browser console

[features]
default = ["mathjax"]
mathjax = ["mathjax_svg", "resvg"]  # native-only; NOT compiled to WASM
```

There are **two binaries** with the same source file:
| Binary | Target | Features | Purpose |
|---|---|---|---|
| `sdof_dynamics` | native | mathjax (default) | Desktop app with LaTeX equations |
| `sdof_dynamics_web` | wasm32-unknown-unknown | none (--no-default-features) | Web app with plain-text equations |

---

## Build Commands

### Native (desktop)
```bash
cargo build --release           # build
cargo run --release             # build + run
```

### WASM (web)
```bash
# Step 1 — build WASM binary
cargo build --target wasm32-unknown-unknown --release \
            --bin sdof_dynamics_web --no-default-features

# Step 2 — generate JS bindings  (wasm-bindgen must match Cargo.lock: 0.2.114)
wasm-bindgen --target web --out-dir _site \
    target/wasm32-unknown-unknown/release/sdof_dynamics_web.wasm

# Step 3 — copy index.html
cp web/index.html _site/index.html

# Step 4 — serve
cd _site && python3 -m http.server 8000
```

The Linux musl wasm-bindgen binary is downloaded by the CI from:  
`https://github.com/rustwasm/wasm-bindgen/releases/download/0.2.114/wasm-bindgen-0.2.114-x86_64-unknown-linux-musl.tar.gz`

### Remote cluster access
The cluster (LUIS HPC, LUH) does not have a display. To test the WASM app locally:
1. Start Python server on the cluster (port 8000)
2. On your local Windows machine: `ssh -L 8000:localhost:8000 luis`
3. Open `http://localhost:8000` in your local browser

---

## IWES App Framework (`shared/template.rs`)

This file is the shared foundation for all IWES egui apps. **Do not modify it unless explicitly asked.**

### Core Types

| Type | Purpose |
|---|---|
| `AppBase` | Shared state in every app: theme, View3D, animation, logos, about dialog |
| `IwesApp` | Trait to implement — provides `title()`, `base()`, `base_mut()`, `controls()`, `content()` |
| `AppWrapper<T>` | Wraps any `IwesApp` into an `eframe::App` |
| `Theme` | Dark/light toggle; provides `bg_color()`, `text_color()`, `accent_color()`, `series_color(i)`, etc. |
| `PanelConfig` | Optional custom panel layout (left width, right panel, tabs) |
| `TabDef` | Tab label definition for multi-tab left panels |

### UI Helpers

| Function | Signature | Notes |
|---|---|---|
| `labeled_slider` | `(ui, label, &mut f64, range, suffix)` | Horizontal label + slider |
| `labeled_slider_step` | `(ui, label, &mut f64, range, suffix, step)` | With explicit drag step |
| `labeled_slider_log` | `(ui, label, &mut f64, range, suffix)` | Logarithmic scale, ideal for wide ranges (0.1–1e6) |
| `section_header` | `(ui, title)` | Separator + heading + separator |
| `vsplit_handle` | `(ui, height) -> f32` | Draggable vertical divider; returns drag delta |
| `hsplit_handle` | `(ui, width) -> f32` | Draggable horizontal divider; returns drag delta |
| `load_texture_from_png` | `(ctx, name, bytes) -> TextureHandle` | Decode PNG bytes → egui texture |

### Layout (rendered by `AppWrapper::update`)

```
┌─────────────────────────────────────────────────────┐
│  [IWES logo]      App Title          [LUH logo][About] │  ← TopBottomPanel "header" (48 px)
├──────────────┬──────────────────────────┬────────────┤
│ Controls     │                          │ Right panel│
│ (left panel) │     CentralPanel         │ (optional) │
│ 260 px wide  │     app.content()        │            │
│ scrollable   │                          │            │
└──────────────┴──────────────────────────┴────────────┘
```

The left panel always draws:
1. "Controls" heading + separator
2. Theme toggle (Dark / Light)
3. Animate + Loop checkboxes
4. Optional tab bar (if `control_tabs()` is implemented)
5. `controls()` or `controls_for_tab(ui, tab_index)`

### WASM Init (`wasm_run_app`)
- Calls `console_error_panic_hook::set_once()` so Rust panics print to the browser console
- Dynamically creates a `<canvas>` element and appends it to `document.body`
- Starts `eframe::WebRunner`

### Creating a New App
1. Create `apps/my_app.rs`
2. Add `[[bin]]` entry in `Cargo.toml`
3. Import: `#[path = "../shared/template.rs"] mod template; use template::*;`
4. Define your struct with `base: AppBase`
5. Implement `IwesApp` (at minimum: `title`, `base`, `base_mut`, `controls`, `content`)
6. Call `run_app::<MyApp>("Title", [1400.0, 900.0])` from `main()`

---

## 3D Viewport (`shared/viewport3d.rs`)

Blender-style camera for apps that need 3D content. Provided as `View3D` inside `AppBase`.

- **Orbit:** left-drag
- **Pan:** middle-drag or Shift+left-drag  
- **Zoom:** scroll wheel
- **FPS mode:** toggle with a key
- **Keyboard presets:** numpad-style view snapping
- **Gizmo:** colored axis lines (X=red, Y=green, Z=blue)

Access via `self.base().view_3d` and `self.base_mut().view_3d`.

---

## The App (`apps/sdof_dynamics.rs`)

### App Struct: `SdofApp`

| Field | Type | Purpose |
|---|---|---|
| `base` | `AppBase` | Framework state |
| `mass`, `stiffness`, `damping` | `f64` | m, k, c (logarithmic sliders, wide ranges) |
| `force_amp`, `excit_freq` | `f64` | F₀ [N], Ω [rad/s] |
| `x0`, `v0` | `f64` | Initial conditions |
| `t_end` | `f64` | Simulation end time |
| `resp_cache` / `resp_hash` | `Option<TimeResponse>` / `u64` | RK4 result cache (invalidated when params change) |
| `cache` | `LatexCache` | MathJax-rendered textures (native only; no-op struct on WASM) |
| `draw_zoom`, `draw_pan` | `f32`, `Vec2` | Schematic zoom/pan state |
| `draw_h_frac`, `time_h_frac` | `f32` | Resizable row fractions for the left column |

### Physics Functions

| Function | Purpose |
|---|---|
| `natural_freq(m, k)` | ωₙ = √(k/m) |
| `damping_ratio(m, k, c)` | ζ = c / (2√(mk)) |
| `damped_freq(wn, zeta)` | ωd = ωₙ√(1−ζ²) |
| `amplification(r, zeta)` | H(r) = 1/√((1−r²)²+(2ζr)²), capped at 10 |
| `phase_lag(r, zeta)` | φ(r) = atan2(2ζr, 1−r²) |
| `rk4_step(...)` | Single RK4 step for m·x''+c·x'+k·x = F₀·cos(Ω·t) |
| `compute_time_response(...)` | Full RK4 integration → `TimeResponse {t, x, v}` |
| `detect_case(zeta, f0, r)` | Classifies response: "Undamped Free", "Resonance", etc. |

### Layout (`content()`)

The central panel uses a **horizontal split** (draggable):

```
Left column (adjustable width):          Right column:
┌───────────────────────────┐            ┌──────────────────┐
│ draw_schematic()          │ ← draw_h   │ draw_equations() │
│  (animated schematic)     │            │  (LaTeX native / │
├───────────────────────────┤            │   plain WASM)    │
│ Transport bar             │            │                  │
│  (Run/Pause/Reset/slider) │            │                  │
├───────────────────────────┤ ← time_h   │                  │
│ draw_time_plot()          │            │                  │
├───────────────────────────┤            │                  │
│ draw_frf_plot()           │            │                  │
└───────────────────────────┘            └──────────────────┘
```

Row heights are fractional and user-adjustable via `hsplit_handle`.  
Column widths use `base.content_split` fraction, adjustable via `vsplit_handle`.

### Controls (left panel, `controls()`)

Sections (in order):
1. **Presets** — 8 wind-turbine-relevant scenarios (buttons)
2. **System Parameters** — m, k, c (log sliders); derived ωₙ, ζ, ωd displayed; case label
3. **Excitation** — F₀ (log slider), Ω slider with "→ res." button, r = Ω/ωₙ slider
4. **Initial Conditions** — x0, v0
5. **Simulation** — t_end, velocity/envelope/force checkboxes
6. **Equation Rendering** — quality/scale sliders for MathJax (native only)

### MathJax Pipeline (native only, `#[cfg(feature = "mathjax")]`)

`mathjax_svg::convert_to_svg(latex)` → SVG string → `resvg` renders to RGBA pixels → `egui::TextureHandle`.  
Cached in `LatexCache` (HashMap keyed by name). Invalidated when theme changes or user clicks "Clear cache".

On WASM: `LatexCache` is a no-op struct; equations use `draw_equations_plain()` with Unicode math symbols.

---

## Web / WASM Details

### `web/index.html` — JavaScript Patches

Two patches are applied before WASM loads (to fix eframe 0.29.x bugs):

**1. Passive event listener fix**  
eframe registers `wheel`/`touchstart`/`touchmove` listeners without `{passive: false}`,  
then calls `preventDefault()`. Chrome blocks this. Fix: override `EventTarget.prototype.addEventListener`  
to force `passive: false` for those event types.

**2. Virtual keyboard / canvas resize fix**  
When eframe's hidden text-agent `<input>` gets focus, the browser triggers a virtual-keyboard  
resize event (even on desktop Windows/Chrome). This causes eframe to temporarily swap canvas  
width ↔ height (portrait flip), producing a white screen.  
Fix: override `Document.prototype.createElement` to add `inputmode="none"` to every `<input>`,  
preventing the virtual keyboard trigger. Also sets `navigator.virtualKeyboard.overlaysContent = true`  
for Chrome 94+ (VirtualKeyboard API).

**Canvas CSS:** `position: fixed; top: 0; left: 0; width: 100vw; height: 100vh`  
(prevents layout shifts from affecting the canvas bounding box)

### Panic Hook
`console_error_panic_hook::set_once()` is called at WASM startup in `wasm_run_app()`.  
Any Rust panic prints a readable stack trace to the browser DevTools console.

---

## CI/CD (`.github/workflows/pages.yml`)

Trigger: push to `main` or manual dispatch.

Steps:
1. `rustup target add wasm32-unknown-unknown`
2. `cargo build --target wasm32-unknown-unknown --release --bin sdof_dynamics_web --no-default-features`
3. Download `wasm-bindgen-0.2.114-x86_64-unknown-linux-musl` from GitHub releases
4. `wasm-bindgen --target web --out-dir _site ...`
5. `cp web/index.html _site/index.html`
6. Deploy `_site/` to GitHub Pages

---

## Known Constraints

- **eframe 0.29.x**: The passive-listener and virtual-keyboard bugs above are specific to this version. If upgrading to 0.30+, verify whether the JS patches in `index.html` are still needed.
- **wasm-bindgen version must match**: `Cargo.lock` locks to `0.2.114`. The `wasm-bindgen` CLI binary used to generate JS must be exactly the same version, or it will refuse to process the WASM.
- **mathjax feature is incompatible with WASM**: `mathjax_svg` and `resvg` use native C/C++ code. Always build the WASM target with `--no-default-features`.
- **LUIS cluster**: Login nodes have a 30-minute job limit. Do not run long builds there without `srun`/`sbatch`. Use SSH port forwarding (`ssh -L 8000:localhost:8000 luis`) to access the served app locally.
