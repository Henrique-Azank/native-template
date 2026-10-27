# native-template

A production-grade, hyper-performant Rust template for native desktop applications that compile seamlessly to WebAssembly. Built with a clean-room, engine-first architecture inspired by **GridCraft** (spreadsheet engine) and **FigCraft** (vector design engine).

---

## Key Features

1. **8-Layer Modular Hierarchy (L0–L7):** Strict downward dependency boundaries verified by `cargo xtask layers`. Zero UI or network crates below Layer 6.
2. **"Everything is a Command" Philosophy:** All application state changes execute through a centralized, panic-isolated, undoable command registry (`Session::execute`).
3. **React / Next.js Developer Ergonomics:**
   - **One file per component/page** in `crates/ui-egui/src/components/` and `crates/ui-egui/src/pages/`.
   - **`use_state` Hook:** React-style local component state for handling uncommitted inputs.
   - **Declarative Router:** Built-in page routing (`Route::Home`, `Route::Details`, `Route::Settings`, etc.).
4. **Asynchronous Network Engine with Dual-Tier Caching:**
   - Background non-blocking HTTP requests.
   - **In-Memory LRU + Persistent Disk Cache** (`~/.cache/template-app/http/`).
   - `StaleWhileRevalidate`, `CacheFirst`, `NetworkOnly`, and `CacheOnly` policies.
5. **Multi-Window Support:**
   - Native OS secondary windows via `eframe` deferred viewports (`show_viewport_deferred`).
   - Shared underlying engine session state with cross-window reactive repainting.
6. **Persistent User Data:** Automatic loading and debounced saving of `AppPrefs` and `UserSession` to `~/.config/template-app/`.
7. **Model Context Protocol (MCP):** Autonomous AI agent control over stdio out of the box (`template-cli mcp`).
8. **100% WebAssembly Compatible:** Core layers compile to `wasm32-unknown-unknown` without OS dependencies.

---

## Workspace Structure

```
native-template/
├── Cargo.toml                       # Workspace definition (resolver="3", unified dependencies & lints)
├── xtask/                           # Automation & CI quality gates
│   └── src/
│       ├── main.rs                  # xtask CLI: ci, layers, wasm
│       └── layers.rs                # Programmatic L0-L7 layer verification
├── crates/
│   ├── core/                        # [L0] Core primitives, IDs, scalar values, error types
│   ├── model/                       # [L1] Document Object Model with persistent COW data structures (imbl/Arc)
│   ├── engine/                      # [L5] Central Session, CommandSpec registry (cmd/*), undo/redo, prefs
│   ├── net/                         # [L6] Async HTTP client, two-tier cache (memory + disk), stale-while-revalidate
│   ├── ui-egui/                     # [L6] Presentation: React-like components, pages, router, multi-window manager
│   └── mcp/                         # [L6] Model Context Protocol server for AI agent automation
└── apps/
    ├── desktop/                     # [L7] Native desktop executable (macOS, Windows, Linux)
    ├── cli/                         # [L7] Headless command line interface & MCP runner (eval, batch commands)
    └── web/                         # [L7] WebAssembly browser client (Trunk / WebGPU / WebGL2)
```

---

## Getting Started

### 1. Run Native Desktop App
```bash
cargo run -p template-desktop
```
- Click **"+ New Window"** to open an independent OS window sharing the same session state.
- Edit an item in one window; observe the other window repainting in real-time.
- Press **Cmd/Ctrl+Z** to test instant $O(1)$ undo/redo snapshots.

### 2. Run Headless CLI & Execute Commands
```bash
# Display document summary
cargo run -p template-cli -- info

# Execute an engine command programmatically
cargo run -p template-cli -- run --cmd 'item.create={"title":"CLI Task","description":"Automated"}'

# Launch the AI Agent Model Context Protocol (MCP) server
cargo run -p template-cli -- mcp
```

### 3. Run WebAssembly Browser Client (Trunk)
```bash
trunk serve apps/web/index.html
```

---

## Quality Gates & Verification

```bash
# 1. Verify architectural layer boundaries
cargo xtask layers

# 2. Check WebAssembly target compilation
cargo xtask wasm

# 3. Run full continuous integration suite (fmt, clippy -D warnings, tests, layers)
cargo xtask ci
```

---

## Architectural Rules

- **Zero UI below L6:** `egui`, `eframe`, `winit`, and `rfd` are strictly prohibited in layers L0 through L5.
- **Zero Networking below L6:** `reqwest`, `tokio`, and `tungstenite` are prohibited in layers L0 through L5.
- **"Never Crash" Standard:** Production code forbids `unsafe` (`unsafe_code = "forbid"`) and denies all `unwrap()`, `expect()`, `panic!()`, `todo!()`, and `unimplemented!()`.
