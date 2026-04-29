# Web Build Instructions for SDOF Dynamics

## Quick Start

The SDOF Dynamics app is fully configured to compile to WebAssembly and run in the browser.

### Automated Build (Recommended)

Simply run the build script:

**Windows:**
```bash
BUILD_WEB.bat
```

**Mac/Linux:**
```bash
bash build_web.sh
```

This will:
1. ✓ Build the WASM binary
2. ✓ Download wasm-bindgen (if needed)
3. ✓ Generate JavaScript bindings
4. ✓ Output the web files to the `web/` directory

### Manual Build (Step-by-step)

#### Step 1: Build the WASM binary

```bash
cargo build --target wasm32-unknown-unknown --release --bin sdof_dynamics_web --no-default-features
```

Creates: `target/wasm32-unknown-unknown/release/sdof_dynamics_web.wasm`

#### Step 2: Generate JavaScript bindings

Download the prebuilt wasm-bindgen binary (no C++ build tools needed):

```bash
# Windows
curl -L -o wasm-bindgen-0.2.114.tar.gz ^
  https://github.com/rustwasm/wasm-bindgen/releases/download/0.2.114/wasm-bindgen-0.2.114-x86_64-pc-windows-msvc.tar.gz
tar -xzf wasm-bindgen-0.2.114.tar.gz

# Generate bindings
wasm-bindgen-0.2.114-x86_64-pc-windows-msvc\wasm-bindgen.exe ^
  --target web --out-dir web ^
  target\wasm32-unknown-unknown\release\sdof_dynamics_web.wasm
```

This creates in the `web/` directory:
- `sdof_dynamics_web.js` — JavaScript module
- `sdof_dynamics_web_bg.wasm` — Binary WebAssembly file (~3.7 MB, gzips to ~1.2 MB)

#### Step 3: Serve the web app

Start a local web server:

```bash
cd web

# Using Python 3 (built-in)
python -m http.server 8000

# Or using Node
npx http-server

# Or using any other static server
```

#### Step 4: Open in browser

Visit: **`http://localhost:8000`**

## Features

- ✅ Full SDOF dynamics UI — theory panel shown with plain-text equations (no LaTeX)
- ✅ Real-time simulation and plotting
- ✅ Zoom and pan controls
- ✅ Animation support
- ✅ Responsive canvas scaling

## Building for Deployment

For production, optimize the WASM:

```bash
# Install wasm-opt (comes with Binaryen)
cargo install wasm-opt

# Optimize the compiled WASM
wasm-opt web/sdof_dynamics_web_bg.wasm -O4 -o web/sdof_dynamics_web_bg_optimized.wasm
mv web/sdof_dynamics_web_bg_optimized.wasm web/sdof_dynamics_web_bg.wasm
```

Then copy the `web/` directory to your web server.

## Troubleshooting

### "wasm-bindgen command not found"
Install with: `cargo install wasm-bindgen-cli`

### Browser shows blank page
1. Check browser console (F12) for errors
2. Ensure all files are in the `web/` directory
3. Check that the HTTP server is serving from the correct directory

### WASM file too large
The WASM binary is ~5MB uncompressed. Most web servers gzip it automatically, reducing it to ~1.5MB.

## Differences from Desktop Version

- **No LaTeX rendering**: The mathjax feature is disabled to avoid V8 JavaScript engine dependency; equations are rendered as plain Unicode text instead
- **Everything else works normally**: Theory panel, all plots, simulations, and controls function identically
