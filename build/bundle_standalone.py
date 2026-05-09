"""Bundle sdof_dynamics_web into a single standalone HTML file.

Reads dist/web/sdof_dynamics_web.js and sdof_dynamics_web_bg.wasm, inlines both
(wasm as base64), and writes dist/sdof_dynamics_standalone.html.

Author: Basem Rajjoub, 2026 — Institut für Windenergiesysteme (IWES), LUH
"""
import base64
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
JS = ROOT / "dist" / "web" / "sdof_dynamics_web.js"
WASM = ROOT / "dist" / "web" / "sdof_dynamics_web_bg.wasm"
OUT = ROOT / "dist" / "sdof_dynamics_standalone.html"

js = JS.read_text(encoding="utf-8")

# Strip the @ts-self-types comment (harmless but unnecessary)
js = re.sub(r"^/\* @ts-self-types[^*]*\*/\s*", "", js)

# Remove the export line — we run as a classic <script>, not a module
js = re.sub(r"^export\s*\{[^}]*\};\s*$", "", js, flags=re.MULTILINE)

# Replace the import.meta.url reference. We force the bytes path instead by
# never letting module_or_path be undefined; this line is only reached if
# the caller passes nothing, but be safe and replace with a clear error.
js = js.replace(
    "module_or_path = new URL('sdof_dynamics_web_bg.wasm', import.meta.url);",
    "throw new Error('standalone bundle: pass wasm bytes to init()');",
)

wasm_b64 = base64.b64encode(WASM.read_bytes()).decode("ascii")

html = f"""<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>SDOF Dynamics (standalone)</title>
<style>
  body {{ margin: 0; padding: 0; overflow: hidden;
         font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; }}
  canvas {{ display: block; width: 100vw; height: 100vh; }}
  #root {{ width: 100%; height: 100vh; }}
  #status {{ position: fixed; top: 50%; left: 50%; transform: translate(-50%, -50%);
            font-size: 14px; color: #444; }}
</style>
</head>
<body>
<div id="root"></div>
<div id="status">Loading WebAssembly...</div>
<script id="wasm-b64" type="application/octet-stream-base64">
{wasm_b64}
</script>
<script>
{js}

// ── standalone bootstrap ──
(function () {{
  function b64ToBytes(b64) {{
    const clean = b64.replace(/\\s+/g, "");
    const bin = atob(clean);
    const out = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
    return out;
  }}
  const status = document.getElementById("status");
  try {{
    const b64 = document.getElementById("wasm-b64").textContent;
    const bytes = b64ToBytes(b64);
    initSync({{ module: bytes }});
    status.remove();
  }} catch (e) {{
    status.textContent = "Failed to start: " + e;
    console.error(e);
  }}
}})();
</script>
</body>
</html>
"""

OUT.write_text(html, encoding="utf-8")
size_mb = OUT.stat().st_size / (1024 * 1024)
print(f"Wrote {OUT} ({size_mb:.2f} MB)")
