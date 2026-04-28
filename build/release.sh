#!/bin/bash
set -e

WASM_BINDGEN_VERSION="0.2.114"
TOOLS_DIR="tools"
BINDGEN_BIN="$TOOLS_DIR/wasm-bindgen"

echo "========================================"
echo "SDOF Dynamics - Release Build"
echo "========================================"
echo

# ── Step 1: Native desktop binary (MathJax) ──────────────────────────────
echo "[1/4] Building native desktop binary (with LaTeX equations)..."
cargo build --release --bin sdof_dynamics --features mathjax
echo "+ Binary: target/release/sdof_dynamics"
echo

# ── Step 2: WebAssembly binary ────────────────────────────────────────────
echo "[2/4] Building WebAssembly binary..."
cargo build --target wasm32-unknown-unknown --release --bin sdof_dynamics_web
echo

# ── Step 3: wasm-bindgen JS bindings ─────────────────────────────────────
echo "[3/4] Generating JavaScript bindings..."
if [ ! -f "$BINDGEN_BIN" ]; then
    echo "    Downloading wasm-bindgen-cli..."
    mkdir -p "$TOOLS_DIR"
    ARCH=$(uname -m)
    OS=$(uname -s)
    if [ "$OS" = "Darwin" ]; then
        TRIPLE="${ARCH}-apple-darwin"
    else
        TRIPLE="${ARCH}-unknown-linux-musl"
    fi
    TARBALL="wasm-bindgen-${WASM_BINDGEN_VERSION}-${TRIPLE}.tar.gz"
    curl -L -o "$TARBALL" \
        "https://github.com/rustwasm/wasm-bindgen/releases/download/${WASM_BINDGEN_VERSION}/${TARBALL}"
    tar -xzf "$TARBALL" -C "$TOOLS_DIR" \
        "wasm-bindgen-${WASM_BINDGEN_VERSION}-${TRIPLE}/wasm-bindgen" \
        --strip-components=1
    rm "$TARBALL"
fi
mkdir -p docs
"$BINDGEN_BIN" --target web --out-dir docs \
    target/wasm32-unknown-unknown/release/sdof_dynamics_web.wasm
echo

# ── Step 4: Copy everything to docs/ ─────────────────────────────────────
echo "[4/4] Staging docs/ for GitHub Pages..."
cp web/index.html docs/index.html
cp target/release/sdof_dynamics docs/sdof_dynamics
echo

echo "========================================"
echo " Release complete — docs/ is ready."
echo "========================================"
echo
echo "  docs/index.html            (web app)"
echo "  docs/sdof_dynamics_web.js  (WASM bindings)"
echo "  docs/sdof_dynamics_web_bg.wasm"
echo "  docs/sdof_dynamics         (Linux/macOS desktop app)"
echo
echo "Commit and push to publish via GitHub Pages."
echo
