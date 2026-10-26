# Framework Basis: High-Performance Native Desktop & WebAssembly Architecture

> **Authoritative Architectural Blueprint**  
> Synthesized from the production architectures of **GridCraft** (spreadsheet engine) and **FigCraft** (vector design engine).  
> Designed as the foundation for the `native-template` repository for hyper-performant, cross-platform, local-first Rust desktop applications that compile seamlessly to WebAssembly.

---

## 1. Executive Summary & Core Architectural Tenets

Both GridCraft and FigCraft represent a disciplined, clean-room approach to building complex creative and productivity applications in Rust. Rather than wrapping web technologies in Electron or building tightly-coupled UI widgets in C++, both applications share an identical core philosophy:

1. **Engine-First, Headless-By-Default:** The entire domain model (formulas, scene graphs, solvers, auto layout, file parsers, dependency graphs) lives in pure Rust libraries completely independent of any windowing system, GPU surface, or GUI toolkit.
2. **"Everything is a Command":** Every user action, script invocation, shortcut, AI agent command, and plugin execution is routed through a centralized command registry (`Session::execute`). Direct mutations of application state from the UI are strictly forbidden.
3. **The "Thin UI" Principle:** The presentation layer (`crates/ui-egui`) is an immediate-mode shell that reads engine state and dispatches commands. It contains zero business logic and can be completely swapped without modifying the engine.
4. **Instant, Zero-Cost Undo/Redo via Structural Sharing:** Application state is stored in persistent, immutable, Copy-on-Write (COW) data structures. Snapshots are cheap $O(1)$ pointer clones.
5. **Never-Crash Standard:** The application is architected to guarantee zero data loss. Unsafe code is forbidden, panics are denied by compiler lints, hostile user inputs are bounded, and runtime commands and UI frames run inside panic guards with automated emergency rollback and recovery.
6. **Native Performance with 100% WebAssembly Compatibility:** The engine, formatting, and rendering pipeline compile to native binaries (macOS, Windows, Linux, BSD) and to browser WebAssembly (`wasm32-unknown-unknown`) without code forks.

---

## 2. The 8-Layer Mental Model (The L0–L7 Standard)

Both codebases enforce a strict 8-layer workspace hierarchy verified by `cargo xtask layers`. Lower layers may never depend on higher layers, and circular dependencies are physically impossible.

```
+---------------------------------------------------------------------------------------------------+
| L7: Concrete Application Deliverables (apps/*)                                                    |
|     Desktop Native App (eframe/wgpu) | Headless CLI & MCP Tool | Web WASM Bundle | Collab Server   |
+---------------------------------------------------------------------------------------------------+
                                                  │
+---------------------------------------------------------------------------------------------------+
| L6: Presentation & External Automation (crates/ui-egui, crates/mcp, crates/collab-net)             |
|     - Immediate-Mode GUI (egui / eframe)                                                          |
|     - Model Context Protocol (MCP) Server for AI Agents                                           |
|     - Real-Time Networking (WebSockets / TLS)                                                      |
+---------------------------------------------------------------------------------------------------+
  ▲ STRICT BOUNDARY: Zero UI (egui/eframe/winit/rfd) and Zero Networking (tungstenite/tokio) below L6!
  │
+---------------------------------------------------------------------------------------------------+
| L5: Application Engine & Facade (crates/engine)                                                   |
|     - Session & Document State Management                                                         |
|     - Command Registry (cmd/*, CommandSpec) & Unified Dispatch                                    |
|     - Undo/Redo Snapshot History & Dirty Tracking                                                 |
|     - Tools, Gestures, Snapping, Clipboard Bridge, Panic Guard                                    |
+---------------------------------------------------------------------------------------------------+
                                                  │
+---------------------------------------------------------------------------------------------------+
| L4: Interchange & Vector Export (crates/svg, crates/pdf)                                          |
|     - PDF Generator / Exporter (krilla)                                                           |
|     - Vector Import / Export (usvg)                                                               |
+---------------------------------------------------------------------------------------------------+
                                                  │
+---------------------------------------------------------------------------------------------------+
| L3: Visualization, Media & Serialization (crates/render, crates/format, crates/chart)             |
|     - Toolkit-Agnostic CPU/GPU Rasterization (tiny-skia retained diff rendering)                  |
|     - Native File Format Package Reading/Writing (ZIP, streamed JSON, embedded assets)           |
|     - Abstract Visualization & Chart Layout Primitives                                            |
+---------------------------------------------------------------------------------------------------+
                                                  │
+---------------------------------------------------------------------------------------------------+
| L2: Domain Systems, Recalculators & Solvers                                                       |
|     - Recalculation Graph / Dependency Engine (GridCraft calc)                                    |
|     - Auto Layout & Constraint Solvers (FigCraft layout)                                          |
|     - Typography & Font Engine (harfrust OpenType shaping, skrifa glyphs, unicode-bidi)           |
|     - Path Booleans & Stroke Geometry (pathops, linesweeper)                                      |
|     - Pure Sync CRDT Core / Multiplayer State Machine (collab)                                    |
|     - Sandboxed WebAssembly Plugin Runtime (wasmi interpreter)                                    |
+---------------------------------------------------------------------------------------------------+
                                                  │
+---------------------------------------------------------------------------------------------------+
| L1: Domain Object Model (DOM) & Expressions                                                        |
|     - Document Model (FigCraft doc: persistent node tree, variables, styles, components)          |
|     - Cell Storage & Style Tables (GridCraft model: 64-row bands, interned styles)                |
|     - Expression Parsers & Evaluators (GridCraft formula / functions: Pratt parser, AST)          |
+---------------------------------------------------------------------------------------------------+
                                                  │
+---------------------------------------------------------------------------------------------------+
| L0: Foundation Primitives                                                                         |
|     - Math, Coordinates & Geometry (geom: Affine, Rect, Point, Bézier curves via kurbo)           |
|     - Color Science & Paint Models (color: sRGB, Display P3, HSB, Gradients, Blend Modes)         |
|     - Scalar Values, Errors & Coercion (core: Value, CellRef, Serials)                            |
|     - Number Formatting Engines (numfmt: Excel-style format strings, sections, conditions)        |
+---------------------------------------------------------------------------------------------------+
```

### Layer Invariant Rules (Automated CI Gates)

1. **Strict Downward Dependency Rule:** A crate at layer $L$ may only depend on crates at layer $< L$.
2. **Explicit Intra-Layer Whitelist:** Crates at the same layer may not depend on each other unless explicitly whitelisted in `xtask/src/layers.rs` (e.g. `color` $\rightarrow$ `geom` in L0; `render` $\rightarrow$ `format` in L3).
3. **The UI Isolation Boundary (`UI_MIN_LAYER = 6`):**
   * Dependencies on `egui`, `eframe`, `winit`, `rfd`, or `wgpu` are forbidden below Layer 6.
   * Layers L0 through L5 are **100% headless, pure Rust libraries**.
4. **The Network Isolation Boundary:**
   * Dependencies on `tungstenite`, `tokio`, `tokio-tungstenite`, `mio`, or `socket2` are forbidden below Layer 6.
   * The domain engine and solvers never touch raw sockets or network stacks directly.
5. **Format Crate Isolation:** File format parsers only parse raw byte slices into memory structures; they are prohibited from calling recalculation solvers or UI allocators during parsing.

---

## 3. Cargo Workspace Configuration & Build System

Both applications use an optimized, multi-target Cargo workspace layout in `Cargo.toml`.

### 3.1 Workspace Metadata & Resolver
```toml
[workspace]
resolver = "3" # Modern 2024 edition resolver behavior
members = ["crates/*", "apps/*", "xtask"]
exclude = ["plugins"] # Plugin SDK builds for wasm32 in its own workspace

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.95"
license = "MIT OR Apache-2.0"
```

### 3.2 Centralized Dependency Management (`[workspace.dependencies]`)
All internal and external dependencies are defined exclusively in the root `Cargo.toml`. Individual crates declare `serde = { workspace = true }` or `app-engine = { workspace = true }`. This guarantees:
* Zero version skew across crates.
* A single unified lockfile (`Cargo.lock`).
* Dramatically reduced compile times and minimal binary size.

### 3.3 Workspace-Wide Lint Policies
The "Never-Crash" standard is enforced at compile time across every crate:
```toml
[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
dbg_macro = "warn"
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
unimplemented = "deny"
unreachable = "deny"
```

### 3.4 Bifurcated Profile Optimizations
To achieve sub-second incremental builds while maintaining fluid 60 FPS developer test runs:
```toml
# Dev builds: fast compilation for app code, high optimization for dependencies
[profile.dev]
opt-level = 1
debug = "line-tables-only"

[profile.dev.package."*"]
opt-level = 2
debug = false

# Desktop release builds: maximum speed
[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 16

# Browser release builds: maximum size optimization
[profile.web-release]
inherits = "release"
opt-level = "s"      # Optimize for size
lto = "fat"          # Cross-crate whole-program optimization
codegen-units = 1    # Maximum dead-code elimination
debug = false
strip = true         # Strip symbol tables and debug info
panic = "abort"      # WASM panic abort reduces binary size
```

---

## 4. The "Everything is a Command" Pattern (Unified Action Bus)

Both GridCraft and FigCraft reject the anti-pattern of embedding business logic inside UI callbacks (`ui.button().clicked()`). Instead, **every state change is an Engine Command**.

### 4.1 The Command Specification (`CommandSpec`)
Defined in `crates/engine/src/cmd/mod.rs`:
```rust
pub struct CommandSpec {
    pub id: &'static str,                    // e.g. "shape.rectangle", "cell.set"
    pub label: &'static str,                 // Human-readable menu/palette label
    pub params: &'static [&'static str],     // Documented JSON parameter names
    pub undoable: bool,                      // Automatically records undo history
    pub journal: bool,                       // Logged for audit/replay/crash logs
    pub enabled: Box<dyn Fn(&Session) -> Result<(), &'static str> + Send + Sync>,
    pub run: Box<dyn Fn(&mut Session, &serde_json::Value) -> Result<serde_json::Value, EngineError> + Send + Sync>,
}
```

### 4.2 Unified Action Bus Architecture
Because all actions pass through `Session::execute(id, params)`:
```
                                +---------------------------+
                                |  UI Buttons / Menus       |
                                +---------------------------+
                                              │ app.run()
                                              ▼
+---------------------+         +---------------------------+         +---------------------+
| Localhost Control   |────────▶|                           |◀────────| Keyboard Shortcuts  |
| Protocol (--control)|         |     Session::execute      |         +---------------------+
+---------------------+         |                           |
                                |  1. Check enabled()       |
+---------------------+         |  2. Capture COW Snapshot  |         +---------------------+
| AI Agents (MCP)     |────────▶|  3. Run in catch_unwind   |◀────────| Pointer Canvas Tools|
+---------------------+         |  4. Update revision / sync|         +---------------------+
                                +---------------------------+
                                              ▲
                                              │
+---------------------+         +-------------┴-------------+
| CLI Headless Scripts|────────▶| Sandboxed WASM Plugins    |
+---------------------+         +---------------------------+
```

### 4.3 Key Invariants of the Command Pattern
1. **Free Undo/Redo:** Commands marked `undoable: true` automatically snapshot the document before execution. If the command succeeds and mutates state, the snapshot is committed to the undo history.
2. **Deterministic Agent Control:** AI agents via MCP and test scripts via `--control <port>` can execute anything a human can do by sending JSON lines over stdio or TCP.
3. **No Dialogs in Commands:** Programmatic command calls never open modal UI windows. If a required parameter is missing, the command returns `Err(EngineError::BadParams)`. Only UI gesture handlers invoke `UiRequest::Dialog`.

---

## 5. State Management & Structural Sharing (COW)

Native creative applications handle huge datasets (1,048,576 spreadsheet rows, or canvas documents with 500,000 vector shapes). Deep copying the document on every edit or keystroke would cripple performance and exhaust memory.

### 5.1 Structural Sharing Mechanics
Both applications use persistent data structures with Copy-on-Write (COW) branching:

* **In GridCraft (`crates/model/src/store.rs`):**
  Worksheets are split into horizontal **bands of 64 rows**, each wrapped in `Arc<Band>`:
  ```rust
  pub struct CellStore {
      bands: BTreeMap<u32, Arc<Band>>,
      len: usize,
  }
  ```
* **In FigCraft (`crates/doc/src/lib.rs`):**
  The scene graph is stored in a persistent Hash-Array Mapped Trie / B-Tree via `imbl`:
  ```rust
  pub struct Document {
      pub pages: Vec<NodeId>,
      nodes: imbl::OrdMap<NodeId, Arc<Node>>,
      dirty: Dirty,
  }
  ```

### 5.2 Benefits of Persistent Structural Sharing
1. **$O(1)$ Undo/Redo Snapshots:** Taking a snapshot requires only cloning the root `Arc<Document>` (copying a single 64-bit pointer).
2. **Minimal Memory Mutation:** Modifying a single cell or moving a vector node clones *only* that specific 64-row band or node path ($O(\log N)$). 99.9% of memory remains shared between history states.
3. **Instant Dirty Checking:** Checking whether a document has unsaved disk changes is a single pointer comparison:
   ```rust
   pub fn is_dirty(&self) -> bool {
       !Arc::ptr_eq(&self.doc, &self.saved_doc)
   }
   ```
4. **Crash Recovery System:** A background thread periodically saves dirty documents to a persistent recovery directory (`~/.config/<app>/Recovery`). If the app is killed or loses power, unsaved work is automatically restored upon launch.

---

## 6. The "Never-Crash" Reliability Standard

Both applications follow the strict "Never Crash" standard defined in `craftrules/standards/never-crash.md`.

```
                  +---------------------------------------------------+
                  |                 Level 1: Compiler                 |
                  |  #![deny(clippy::unwrap_used, clippy::panic...)]  |
                  |  unsafe_code = "forbid"                           |
                  +---------------------------------------------------+
                                            │
                                            ▼
                  +---------------------------------------------------+
                  |                 Level 2: Defensive                |
                  |  Input-derived numbers are hostile                |
                  |  get() instead of [idx], saturating arithmetic    |
                  |  Bounded recursion & capped allocation            |
                  +---------------------------------------------------+
                                            │
                                            ▼
                  +---------------------------------------------------+
                  |                 Level 3: Command Guard            |
                  |  Session::execute wrapped in catch_unwind         |
                  |  Rolls back to COW snapshot on panic              |
                  +---------------------------------------------------+
                                            │
                                            ▼
                  +---------------------------------------------------+
                  |                 Level 4: UI Frame Guard           |
                  |  windows::guard wraps every UI frame              |
                  |  Emergency save to ~/.config/<app>/Recovery       |
                  +---------------------------------------------------+
```

### 6.1 Defensive Coding Patterns
* **No Raw Indexing:** Array indexing (`items[i]`) is replaced with `items.get(i).ok_or(...)`.
* **Bounded Allocations:** All allocations derived from user input or file headers (e.g. range sizes, `REPT()` counts, image dimensions) are strictly capped.
* **Safe Date/Time:** Hand-rolled calendar math is forbidden; all calendar calculations pass through `chrono` or `time`.

### 6.2 Multi-Tiered Panic Containment
1. **Command Panic Isolation:**
   ```rust
   let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
       (spec.run)(self, &params)
   }));
   if let Err(panic_payload) = res {
       // Roll back document and selection to pre-command snapshot
       self.doc = before.doc;
       self.selection = before.selection;
       return Err(EngineError::Internal { cmd: spec.id.to_string(), msg: "Panicked".into() });
   }
   ```
2. **Frame-Level Last Resort (`windows::guard`):**
   If an unexpected panic escapes during UI painting, `windows::guard` intercepts the unwind, displays a diagnostic toast, and triggers `emergency_save()` to save recovery copies of all open files to disk before the window closes.

---

## 7. GUI Framework & Platform Shell Overrides

Both applications use [`egui 0.36`](https://github.com/emilk/egui) and [`eframe 0.36`](https://github.com/emilk/egui/tree/master/crates/eframe) running on [`wgpu`](https://wgpu.rs). However, neither app uses default out-of-the-box egui behavior. They implement a standard set of desktop shell customizations:

### 7.1 Window Frame Negotiation (`craft-window`)
To avoid inconsistent window frames across operating systems, applications negotiate window chrome dynamically:
* **macOS:** Renders a fullsize content view with native traffic lights inset into the custom tab strip (`TitleBar::Inset`).
* **Windows:** Disables OS decorations (`TitleBar::Custom`) to draw a frameless, integrated dark/light title bar with minimize/maximize/close controls matching the application palette.
* **Linux / BSD:** Probes the running desktop. Compositors providing Server-Side Decorations (KDE KWin, Sway, Hyprland, COSMIC) keep native decorations (`TitleBar::Native`). Compositors without SSD (GNOME Mutter under Wayland) automatically switch to the custom title bar (`TitleBar::Custom`).

### 7.2 Windows GPU Driver Safety & Fallbacks
In `apps/<app>/src/graphics.rs`:
* Forces the **FXC shader compiler** on DirectX 12 to bypass driver hangs on legacy GPUs.
* Evaluates GPU adapters: DirectX 12 $\rightarrow$ OpenGL $\rightarrow$ WARP software rasterizer.
* Explicitly disables Vulkan by default on Windows to prevent crashes caused by buggy legacy Intel graphics drivers (`igvk64.dll`).

### 7.3 Apple Events Interception (`fmv-macos-events`)
Standard `winit` does not handle macOS `kAEOpenDocuments` events. Both apps embed `fmv-macos-events` to ensure that double-clicking documents in Finder, dragging onto the Dock icon, or opening via `open -a <App> <file>` opens files correctly.

### 7.4 Keyboard Zoom Hijack Suppression
Egui's default `Ctrl/Cmd + Plus / Minus / 0` zooms the entire UI. Both applications disable this behavior (`ctx.options_mut(|o| o.zoom_with_keyboard = false)`) and route those keys to canvas/spreadsheet viewport zoom commands.

### 7.5 Synthetic Input Injection for Agent Testing
Both applications override `eframe::App::raw_input_hook` to inject batches of synthetic pointer and keyboard events from the localhost control channel (`ui.input`), enabling automated UI testing via `egui_kittest` and pixel-perfect screenshot verification without simulated mouse hardware.

### 7.6 Automation Roots Confinement
When running with `--automation-read-root <dir>` and `--automation-write-root <dir>`, file access is strictly sandboxed. Every UI frame and agent command runs inside `file_access::confined`, preventing arbitrary disk traversal.

---

## 8. Concurrency, Multi-Threading & Asynchronous Operations

Both applications adopt an **asynchronous worker thread model** where the UI thread remains dedicated to 60 FPS interactions:

```
+-------------------------------------------------------------------------------+
| Main UI Thread (winit event loop)                                             |
|  - Processes gestures, pointer tools, and immediate-mode UI rendering          |
|  - Executes Session::execute commands                                         |
+-------------------------------------------------------------------------------+
         │                                       ▲
         │ 1. Dispatches background jobs         │ 2. Wakes UI via request_repaint()
         ▼                                       │
+-------------------------------------------------------------------------------+
| Dedicated Background Worker Threads                                           |
|  • "render": Off-thread CPU rasterization (tiny-skia) with retained pixmaps   |
|  • "font-scan": Non-blocking system font discovery & OpenType table indexing   |
|  • "save": Background ZIP serialization & compression for auto-save            |
|  • "control": TCP listener for localhost JSON-RPC agent control               |
|  • "live-hub" / "live-conn": Real-time WebSocket CRDT synchronization         |
+-------------------------------------------------------------------------------+
```

### 8.1 Key Concurrency Patterns
1. **Zero Polling:** Background worker threads communicate over `std::sync::mpsc` channels and wake the UI thread via `ctx.request_repaint()` or `ctx.request_repaint_of(viewport)`. When idle, the application sleeps and draws 0 frames, consuming 0% CPU.
2. **Progressive Preview Refinement:** If full renders exceed a frame threshold (e.g. 12ms), the worker immediately returns a **4× downsampled coarse preview** (`PREVIEW_FACTOR = 4.0`), followed by the refined full-resolution pixmap.
3. **Retained Diff Rendering:** Panning reuses the existing pixmap and renders only the newly uncovered edges. Editing re-renders only the dirty bounding rectangles of modified layers.

---

## 9. WebAssembly (WASM) & Cross-Platform Parity Strategy

Both applications deploy to web browsers via WebAssembly using the exact same domain code:

### 9.1 The Web Parity Invariants
1. **Pure Rust L0–L5:** Core models, calculation graphs, formatting, and vector renderers never touch OS primitives (no `std::fs`, no raw threads, no C libraries).
2. **Abstracted I/O:** File operations accept `&[u8]` byte slices or async futures.
3. **Abstracted Clocks:** Time calculations use `web_time::Instant` (standard clock natively, browser high-res performance clock in WASM).
4. **Abstracted Networking:** Local TCP sockets (`tungstenite`) on desktop; browser WebSockets (`web_sys::WebSocket`) on WASM.
5. **Continuous Web Verification:** `cargo xtask wasm` compiles all core crates against `wasm32-unknown-unknown` on every CI run.

---

## 10. Blueprint for the `native-template` Repository

Based on the shared architecture of GridCraft and FigCraft, a production-grade template repository must be organized as follows:

### 10.1 Recommended Workspace Directory Structure

```
native-template/
├── Cargo.toml                  # Workspace definition, resolver="3", unified deps & lints
├── Cargo.lock
├── xtask/                      # Architecture & CI validation tools
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs             # xtask CLI: ci, layers, wasm
│       └── layers.rs           # Programmatic L0-L7 layer verification
├── crates/
│   ├── core/                   # L0: Fundamental types, math, coordinates, units
│   ├── color/                  # L0: Color spaces (sRGB, P3), paints, blend modes
│   ├── model/                  # L1: Document object model, persistent tree, COW stores
│   ├── calc/                   # L2: Domain solvers, algorithms, calculations
│   ├── text/                   # L2: HarfBuzz shaping (harfrust), skrifa glyphs
│   ├── render/                 # L3: CPU rasterization (tiny-skia), retained caching
│   ├── format/                 # L3: Native file serialization (.template ZIP/JSON)
│   ├── engine/                 # L5: Session, CommandSpec registry (cmd/*), history
│   ├── ui-egui/                # L6: Immediate-mode UI, theme tokens, canvas worker
│   └── mcp/                    # L6: Model Context Protocol server for AI agents
└── apps/
    ├── desktop/                # L7: Native desktop app (eframe, wgpu, craft-window)
    ├── cli/                    # L7: Headless CLI & MCP runner
    └── web/                    # L7: WASM browser client (Trunk / eframe WebRunner)
```

### 10.2 Starter Files & Templates

#### 1. Root `Cargo.toml`
```toml
[workspace]
resolver = "3"
members = ["crates/*", "apps/*", "xtask"]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.95"
license = "MIT OR Apache-2.0"

[workspace.dependencies]
# Internal workspace crates
template-core = { path = "crates/core" }
template-model = { path = "crates/model" }
template-engine = { path = "crates/engine" }
template-ui-egui = { path = "crates/ui-egui" }

# Foundations
serde = { version = "1", features = ["derive", "rc"] }
serde_json = { version = "1", features = ["float_roundtrip"] }
thiserror = "2"
log = "0.4"
env_logger = "0.11"
imbl = { version = "7", features = ["serde"] }
web-time = "1"

# Graphics & UI
egui = "0.36"
eframe = { version = "0.36", default-features = false }
tiny-skia = "0.12"

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
unimplemented = "deny"
unreachable = "deny"

[profile.dev]
opt-level = 1
debug = "line-tables-only"

[profile.dev.package."*"]
opt-level = 2
debug = false

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 16

[profile.web-release]
inherits = "release"
opt-level = "s"
lto = "fat"
codegen-units = 1
debug = false
strip = true
panic = "abort"
```

#### 2. Layer Checker (`xtask/src/layers.rs`)
```rust
//! Validates that no crate violates the L0-L7 downward dependency rule.

use std::collections::BTreeMap;
use std::process::Command;

const TABLE: &[(&str, u8)] = &[
    ("core", 0),
    ("color", 0),
    ("model", 1),
    ("calc", 2),
    ("text", 2),
    ("render", 3),
    ("format", 3),
    ("engine", 5),
    ("ui-egui", 6),
    ("mcp", 6),
];

pub fn check(root: &std::path::Path) -> bool {
    let out = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .current_dir(root)
        .output()
        .expect("cargo metadata failed");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("invalid json");
    let layer: BTreeMap<String, u8> = TABLE.iter().map(|(n, l)| (format!("template-{n}"), *l)).collect();
    let mut ok = true;

    for pkg in v["packages"].as_array().cloned().unwrap_or_default() {
        let name = pkg["name"].as_str().unwrap_or("").to_string();
        let Some(&l) = layer.get(&name) else { continue };
        for dep in pkg["dependencies"].as_array().cloned().unwrap_or_default() {
            let dn = dep["name"].as_str().unwrap_or("");
            if dep["kind"].as_str() == Some("dev") { continue; }
            if l < 6 && (dn == "egui" || dn == "eframe" || dn == "rfd" || dn == "winit") {
                eprintln!("layer violation: {name} (L{l}) depends on UI crate {dn}");
                ok = false;
            }
            if let Some(&dl) = layer.get(dn) {
                if dl >= l {
                    eprintln!("layer violation: {name} (L{l}) depends on {dn} (L{dl})");
                    ok = false;
                }
            }
        }
    }
    ok
}
```

#### 3. Command Specification & Engine Session (`crates/engine/src/lib.rs`)
```rust
use std::sync::Arc;
use serde_json::Value as Json;

pub struct CommandSpec {
    pub id: &'static str,
    pub label: &'static str,
    pub undoable: bool,
    pub run: Box<dyn Fn(&mut Session, &Json) -> Result<Json, String> + Send + Sync>,
}

pub struct HistoryEntry {
    pub doc: Arc<template_model::Document>,
}

pub struct Session {
    pub doc: Arc<template_model::Document>,
    saved_doc: Arc<template_model::Document>,
    pub undo: Vec<HistoryEntry>,
    pub redo: Vec<HistoryEntry>,
    commands: std::collections::BTreeMap<&'static str, CommandSpec>,
}

impl Session {
    pub fn execute(&mut self, id: &str, params: &Json) -> Result<Json, String> {
        let spec = self.commands.get(id).ok_or_else(|| format!("unknown command {id}"))?;
        let before = self.doc.clone();
        
        // Execute inside panic guard
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            (spec.run)(self, params)
        }));

        match res {
            Ok(Ok(result)) => {
                if spec.undoable && !Arc::ptr_eq(&self.doc, &before) {
                    self.undo.push(HistoryEntry { doc: before });
                    self.redo.clear();
                }
                Ok(result)
            }
            Ok(Err(err)) => {
                self.doc = before; // Roll back on application error
                Err(err)
            }
            Err(_) => {
                self.doc = before; // Roll back on panic
                Err(format!("internal error in {id}"))
            }
        }
    }
}
```

---

## 11. Architectural Comparison Summary Matrix

| Architectural Dimension | GridCraft (Spreadsheet Engine) | FigCraft (Vector Design Engine) | `native-template` Standard |
|---|---|---|---|
| **Primary Domain Model** | 2D banded sparse grid (64-row bands), interned style tables | Persistent scene graph tree (`imbl::OrdMap`), auto layout | Persistent immutable data structures with structural sharing |
| **Recalculation / Solvers** | Dependency DAG topological recalculator, dynamic array spill | Auto layout solver, path boolean sweeper, variable mode resolver | Headless solver crates isolated at Layer L2 |
| **Rendering Strategy** | Virtualized direct drawing (`egui::Painter`), 4-quadrant freeze panes | Retained CPU rendering worker (`tiny-skia`) with coarse preview scaling | Isolated rendering crate (L3) with worker thread offload |
| **State Mutation Pattern** | Unified command registry (`cmd/*`), JSON parameters | Unified command registry (`cmd/*`), JSON parameters | **"Everything is a Command"** via `Session::execute` |
| **Undo / Redo** | $O(1)$ snapshots of `Arc<Workbook>` banded cell store | $O(1)$ snapshots of `Arc<Document>` persistent HAMT map | Instant $O(1)$ pointer clones of root document `Arc` |
| **Reliability & Crash Guard** | `Session::execute` catch-unwind, fuzzing suites | `Session::execute` catch-unwind, `windows::guard` frame guard | Multi-tiered panic containment + emergency disk save |
| **Platform Shell Overrides** | `craft-window` titlebars, `fmv-macos-events`, GPU probe | `craft-window` titlebars, `fmv-macos-events`, GPU probe | Shared desktop chrome negotiation & driver fallbacks |
| **AI Agent Support** | Model Context Protocol (`crates/mcp`), JSON-RPC control port | Model Context Protocol (`crates/mcp`), JSON-RPC control port | First-class MCP server & localhost control socket |
| **WebAssembly Strategy** | Pure L0–L5, WebGPU/WebGL2 fallback via Trunk | Pure L0–L5, WebGPU/WebGL2 fallback via Trunk | Continuous `cargo xtask wasm` verification |

---

### Conclusion
By adopting this shared architectural base, any new project created with `native-template` inherits:
1. **Immunity from architectural rot** via automated layer checking.
2. **True cross-platform performance** (native 60+ FPS desktop and instant-loading browser WASM).
3. **Enterprise reliability** with zero panics, automatic undo/redo, and background crash recovery.
4. **Built-in agentic control** through the Model Context Protocol (MCP) and JSON-RPC control interfaces.
