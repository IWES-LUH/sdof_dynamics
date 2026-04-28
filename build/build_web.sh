#!/bin/bash
set -e

WASM_BINDGEN_VERSION="0.2.114"
TOOLS_DIR="tools"
BINDGEN_BIN="$TOOLS_DIR/wasm-bindgen"

echo "========================================"
echo "SDOF Dynamics - WebAssembly Build"
echo "========================================"
echo

# Step 1: Build WASM
echo "[1/3] Building WebAssembly binary..."
cargo build --target wasm32-unknown-unknown --release --bin sdof_dynamics_web
echo "✓ WASM build complete"
echo

# Step 2: Download wasm-bindgen for the current platform if not present
if [ ! -f "$BINDGEN_BIN" ]; then
    echo "[2/3] Downloading wasm-bindgen..."
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
echo "✓ wasm-bindgen ready"
echo

# Step 3: Generate JavaScript bindings
echo "[3/3] Generating JavaScript bindings..."
mkdir -p dist/web
"$BINDGEN_BIN" --target web --out-dir dist/web \
    target/wasm32-unknown-unknown/release/sdof_dynamics_web.wasm
echo "✓ Bindings generated"
echo

# Step 4: Stage index.html next to the generated JS/WASM
cp web/index.html dist/web/index.html
echo "✓ index.html staged"
echo

echo "========================================"
echo "✓ Build complete!"
echo "========================================"
echo
echo "Next: Run the web server"
echo
echo "  cd dist/web"
echo "  python3 -m http.server 8000"
echo
echo "Then open: http://localhost:8000"
echo
