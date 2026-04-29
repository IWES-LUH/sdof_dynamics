@echo off
setlocal enabledelayedexpansion

echo ========================================
echo SDOF Dynamics - Release Build
echo ========================================
echo.

REM ── Step 1: Native desktop binary (MathJax) ──────────────────────────────
echo [1/4] Building native desktop binary (with LaTeX equations)...
cargo build --release --bin sdof_dynamics --features mathjax
if errorlevel 1 ( echo Build failed! && exit /b 1 )
echo + Binary: target\release\sdof_dynamics.exe
echo.

REM ── Step 2: WebAssembly binary ────────────────────────────────────────────
echo [2/4] Building WebAssembly binary...
cargo build --target wasm32-unknown-unknown --release --bin sdof_dynamics_web --no-default-features
if errorlevel 1 ( echo WASM build failed! && exit /b 1 )
echo.

REM ── Step 3: wasm-bindgen JS bindings ─────────────────────────────────────
echo [3/4] Generating JavaScript bindings...
if not exist "tools\wasm-bindgen-0.2.114-x86_64-pc-windows-msvc\wasm-bindgen.exe" (
    echo     Downloading wasm-bindgen-cli...
    curl -L -o wasm-bindgen-0.2.114.tar.gz ^
        https://github.com/rustwasm/wasm-bindgen/releases/download/0.2.114/wasm-bindgen-0.2.114-x86_64-pc-windows-msvc.tar.gz
    tar -xzf wasm-bindgen-0.2.114.tar.gz -C tools
    del wasm-bindgen-0.2.114.tar.gz
)
tools\wasm-bindgen-0.2.114-x86_64-pc-windows-msvc\wasm-bindgen.exe ^
    --target web --out-dir docs ^
    target\wasm32-unknown-unknown\release\sdof_dynamics_web.wasm
if errorlevel 1 ( echo wasm-bindgen failed! && exit /b 1 )
echo.

REM ── Step 4: Copy everything to docs\ ─────────────────────────────────────
echo [4/4] Staging docs\ for GitHub Pages...
copy /Y web\index.html docs\index.html >nul
copy /Y target\release\sdof_dynamics.exe docs\sdof_dynamics.exe >nul
echo.

echo ========================================
echo  Release complete — docs\ is ready.
echo ========================================
echo.
echo   docs\index.html            (web app)
echo   docs\sdof_dynamics_web.js  (WASM bindings)
echo   docs\sdof_dynamics_web_bg.wasm
echo   docs\sdof_dynamics.exe     (Windows desktop app)
echo.
echo Commit and push to publish via GitHub Pages.
echo.
pause
