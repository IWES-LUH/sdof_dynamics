#!/bin/bash
set -e

echo "========================================"
echo "SDOF Dynamics - WebAssembly Build"
echo "========================================"
echo

# Step 1: Build WASM
echo "[1/3] Building WebAssembly binary..."
cargo build --target wasm32-unknown-unknown --release --bin sdof_dynamics_web
echo "✓ WASM build complete"
echo

# Step 2: Download wasm-bindgen if not present
if [ ! -f "tools/wasm-bindgen-0.2.114-x86_64-pc-windows-msvc/wasm-bindgen.exe" ]; then
    echo "[2/3] Downloading wasm-bindgen..."
    curl -L -o wasm-bindgen-0.2.114.tar.gz \
        https://github.com/rustwasm/wasm-bindgen/releases/download/0.2.114/wasm-bindgen-0.2.114-x86_64-pc-windows-msvc.tar.gz
    tar -xzf wasm-bindgen-0.2.114.tar.gz
fi
echo "✓ wasm-bindgen ready"
echo

# Step 3: Generate JavaScript bindings
echo "[3/3] Generating JavaScript bindings..."
./tools/wasm-bindgen-0.2.114-x86_64-pc-windows-msvc/wasm-bindgen.exe \
    --target web --out-dir dist/web \
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
echo "  python -m http.server 8000"
echo
echo "Then open: http://localhost:8000"
echo
