# FigCraft — Project Architecture Overview

## 1. Executive Summary

FigCraft is an open-source, clean-room, Rust-native vector graphics and interface design application engineered for **Figma parity and superiority** (speed, privacy, agent control, local-first operation, and true desktop execution). It is architected around an **engine-first, command-driven design** where core design scene graphs, auto layout solvers, vector geometry algorithms, real-time multiplayer CRDTs, and file parsers are strictly decoupled from the presentation layer.

FigCraft runs natively across macOS, Windows, Linux, and BSD via native desktop executables, in web browsers via WebAssembly (WASM), and on servers as a headless collaboration hub.

---

## 2. Rust GUI Framework & Platform Shell

### 2.1 Framework Used
* **Primary GUI Toolkit:** [`egui`](https://github.com/emilk/egui) (version **0.36**), an immediate-mode GUI framework written in pure Rust.
* **Application Shell & Windowing Runner:** [`eframe`](https://github.com/emilk/egui/tree/master/crates/eframe) (version **0.36**), which wraps [`winit`](https://github.com/rust-windowing/winit) (version **0.30**) for cross-platform window lifecycle, pointer events, and display scaling.
* **Hardware Graphics Backend:** [`wgpu`](https://wgpu.rs) (WebGPU abstraction layer) as the primary GPU surface renderer across desktop platforms:
  * **macOS:** Metal backend.
  * **Windows:** DirectX 12 by default (with automated fallback to OpenGL/WARP).
  * **Linux / BSD:** Vulkan and OpenGL backends (with Wayland and X11 support).
  * **Web (WASM):** WebGPU with automatic fallback to WebGL2 via `trunk`.
* **Multi-Window Desktop Architecture:** FigCraft natively supports multi-window workflows on the desktop (File ▸ New Window, dragging tabs off the tab bar, or opening files into independent OS windows). Implemented in `crates/ui-egui/src/windows.rs` using `egui` deferred viewports (`ViewportId`, `ctx.show_viewport_deferred`) where all windows share a common session and thread-local application instance (`with_app`), while each window maintains its own viewport geometry, UI panels, and dedicated canvas render worker.
* **Accessibility:** Integrated through [`accesskit`](https://accesskit.dev) across all UI panels, controls, and canvas objects.

### 2.2 Framework Decoupling & Thin UI Principle
The presentation layer (`crates/ui-egui`) sits at layer L6 and depends only on `crates/engine`, `crates/render`, and foundation crates. Layering is enforced by `cargo xtask layers`. **Nothing below L6 depends on `egui`, `eframe`, `winit`, or `rfd`**. The UI is a completely swappable frontend over the headless engine.

---

## 3. Framework Modifications, Workarounds & Overrides

FigCraft does not merely use vanilla `egui` and `eframe`. It implements several platform adaptations, crash guards, and driver workarounds to achieve professional design tool fidelity:

### 3.1 Window Frame Negotiation via `craft-window`
Instead of hardcoding client-side decorations or relying on disparate platform logic, FigCraft integrates the shared [`craft-window`](https://github.com/storytold/craft-libs) protocol (`apps/figcraft/src/main.rs`):
* **macOS (`TitleBar::Inset`):** Keeps the native macOS window frame with native traffic light buttons inset directly into FigCraft's custom tab bar (`viewport.with_fullsize_content_view(true).with_titlebar_shown(false).with_title_shown(false)`).
* **Windows (`TitleBar::Custom`):** Disables system decorations (`with_decorations(false)`) and renders FigCraft's custom dark/light title bar with custom window minimize, maximize/restore, and close buttons, seamless with the toolbar.
* **Linux / BSD (`TitleBar::Native` vs `TitleBar::Custom`):** Dynamically probes the running desktop environment. Compositors providing Server-Side Decorations (SSD) like KDE Plasma (KWin), Sway, Hyprland, and COSMIC retain native system frames (`TitleBar::Native`), whereas compositors without SSD (such as GNOME Mutter under Wayland) automatically switch to FigCraft's custom title bar (`TitleBar::Custom`). This replaces legacy brittle `xdg-decoration` protocol probes.

### 3.2 Windows GPU Driver Safety & Shader Compilation
In `apps/figcraft/src/graphics.rs`, FigCraft inspects GPU adapters prior to initializing `eframe`:
* Forces the **FXC shader compiler** backend on DirectX 12 to eliminate intermittent driver compilation hangs on older GPUs.
* Prefers DirectX 12, with automatic fallbacks to OpenGL or WARP software rasterization.
* Intentionally avoids Vulkan on Windows by default to prevent crashes observed with legacy Intel UHD graphics drivers (`igvk64.dll`).

### 3.3 Apple Events Interception (`fmv-macos-events`)
Standard `winit 0.30` does not dispatch macOS Apple Events. To ensure that double-clicking a `.figcraft` or `.fig` file in Finder, dragging a file onto the Dock icon, or opening via `open -a FigCraft <file>` correctly opens files inside the running instance, FigCraft embeds `fmv-macos-events` (`apps/figcraft/src/apple_events.rs`), capturing `kAEOpenDocuments` events before and during the event loop.

### 3.4 Keyboard Zoom Hijack Suppression
Standard `egui` intercepts `Cmd/Ctrl + Plus / Minus / 0` to scale the entire application interface. In a design tool, these keystrokes must zoom the design canvas (`view.zoomIn`, `view.zoomOut`, `view.zoom100`). FigCraft disables egui's default keyboard zoom (`ctx.options_mut(|o| o.zoom_with_keyboard = false)`) and handles canvas zooming through engine commands and smooth exponential zoom scaling.

### 3.5 Automation Roots Confinement (`--automation-read-root` / `--automation-write-root`)
To safely host automated AI agents and untrusted script execution, `figcraft_engine::file_access` implements file sandboxing (`apps/figcraft/src/main.rs`). Every UI frame, command execution, and control channel request is wrapped in `figcraft_engine::file_access::confined`, strictly blocking file reads or writes outside designated root directories while permitting internal app configs (settings, crash recovery, plugins).

### 3.6 Synthetic Input Injection for Headless Testing & Agent Control
FigCraft overrides `eframe::App::raw_input_hook` via `windows::take_input`. Queued batches of synthetic pointer and keyboard events sent over the localhost JSON-RPC control protocol (`ui.input`) are seamlessly merged into the window's raw input pipeline, enabling full programmatic driving and pixel-perfect screenshot verification.

### 3.7 Frame Panic Guard & Emergency Recovery
Every UI frame is executed inside `windows::guard`:
```rust
std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    figcraft_engine::file_access::confined(roots.as_ref(), || f(&mut *app))
}))
```
If an unhandled panic occurs within any UI component or panel, the application catches the unwind, displays a diagnostic toast alert, and invokes `app.services.emergency_save(&app.session)` to guarantee that unsaved files are preserved in the recovery cache (`~/.config/figcraft/Recovery`) rather than losing work.

---

## 4. Application Core Segmentation & Layering

The FigCraft codebase is organized into a strictly layered architecture governed by `xtask/src/layers.rs`. A crate at layer $L$ may **only depend on crates at strictly lower layers** ($< L$).

```
+---------------------------------------------------------------------------------------------------+
| L7: Applications                                                                                  |
|     apps/figcraft (Desktop) | apps/figcraft-cli (CLI/MCP) | apps/figcraft-web | figcraft-server    |
+---------------------------------------------------------------------------------------------------+
                                                  |
+---------------------------------------------------------------------------------------------------+
| L6: Presentation & Networking                                                                     |
|     crates/ui-egui (UI3 Desktop Interface) | crates/mcp (Agent Protocol) | crates/collab-net (WS) |
+---------------------------------------------------------------------------------------------------+
                                                  |
+---------------------------------------------------------------------------------------------------+
| L5: Application Engine & Facade                                                                   |
|     crates/engine (Session, Command Registry cmd/*, Snapping, Tools, Components Sync, History)   |
+---------------------------------------------------------------------------------------------------+
                                                  |
+---------------------------------------------------------------------------------------------------+
| L4: Vector Formats & Document Interchange                                                         |
|     crates/svg (usvg Import/Export) | crates/pdf (krilla Vector PDF Exporter)                     |
+---------------------------------------------------------------------------------------------------+
                                                  |
+---------------------------------------------------------------------------------------------------+
| L3: Rasterization & Document Serialization                                                        |
|     crates/render (tiny-skia CPU Retained Rasterizer) | crates/format (.figcraft zip/JSON & REST)  |
+---------------------------------------------------------------------------------------------------+
                                                  |
+---------------------------------------------------------------------------------------------------+
| L2: Document Systems & Sandboxed Execution                                                        |
|     crates/layout (Auto Layout) | crates/text (HarfBuzz/skrifa) | crates/pathops (Booleans)      |
|     crates/collab (CRDT/Multiplayer Core) | crates/plugin (wasmi WebAssembly Metered Sandbox)     |
+---------------------------------------------------------------------------------------------------+
                                                  |
+---------------------------------------------------------------------------------------------------+
| L1: Document Object Model                                                                         |
|     crates/doc (Persistent Node Tree imbl, Stable NodeId, Variables, Styles, Components)          |
+---------------------------------------------------------------------------------------------------+
                                                  |
+---------------------------------------------------------------------------------------------------+
| L0: Foundation Primitives                                                                         |
|     crates/geom (Curves, Transforms, kurbo) | crates/color (P3, HSB, Paints, Gradients, Blends)    |
+---------------------------------------------------------------------------------------------------+
```

### Detailed Crate Responsibilities

| Layer | Crate | Primary Responsibilities |
|---|---|---|
| **L0** | `crates/geom` | Geometric foundations built on `kurbo 0.13`: 2D affine transforms, point/rect/size primitives, Bézier curves, smoothed squircle corners, polygons, stars, arcs, and vector projection. |
| **L0** | `crates/color` | Color representations: HSB/HSL, sRGB, Display P3 wide gamut, hex/CSS parsing, paint models (solid, linear, radial, angular, diamond gradients, image fills), and 16 blend modes (Multiply, Screen, Overlay, etc.). |
| **L1** | `crates/doc` | The core Figma-shaped document model: pages, immutable node tree in a persistent map (`imbl::OrdMap<NodeId, Arc<Node>>`), stable node IDs, components, component sets, instances, styles, variables, modes, prototype graphs, comments, and version metadata. |
| **L2** | `crates/layout` | High-performance Auto Layout engine: horizontal, vertical, wrap, and grid layouts; sizing modes (`Fixed`, `Hug`, `Fill`); min/max dimensions; gaps; padding; absolute positioning within auto layout; constraints evaluation. |
| **L2** | `crates/text` | Typography and text layout engine: font database, OpenType text shaping via [`harfrust`](https://github.com/RazrFalcon/harfrust), bidirectional text (`unicode-bidi`), line breaking, variable font axes, and glyph outline parsing via [`skrifa`](https://github.com/googlefonts/fontations). |
| **L2** | `crates/pathops` | Path boolean operations (Union, Subtract, Intersect, Exclude) using [`linesweeper`], outline stroke generation with inner/center/outer alignment, miters, and arrowheads. |
| **L2** | `crates/collab` | Pure, network-agnostic real-time multiplayer core: synchronized object/property model, changesets, fractional indexing, server ordering, client reconciliation, and wire serialization. |
| **L2** | `crates/plugin` | Sandboxed WebAssembly plugin runtime using [`wasmi`] (pure-Rust metered interpreter; memory capped, zero host filesystem/network/clock access, portable deterministic dispatch). |
| **L3** | `crates/render` | Pure CPU rendering pipeline built on [`tiny-skia`]: isolated layers for opacity and blend modes, clipping masks, drop/inner shadows, blurs (uniform, progressive, background glass), texture/noise, text rendering, and retained diff-only caching (`figcraft_render::Retained`). |
| **L3** | `crates/format` | Native `.figcraft` document package reading and writing (ZIP archive containing streamed `document.json` plus deduplicated SHA-256 image assets), version history, and Figma REST API JSON importer. |
| **L4** | `crates/svg` | Bidirectional SVG import and export using [`usvg`]. |
| **L4** | `crates/pdf` | Vector PDF export using [`krilla`] with CMYK/RGB color profiles, embedded subset fonts, and print marks. |
| **L5** | `crates/engine` | Central application engine: `Session` state, undo/redo history ($O(1)$ snapshots), command catalog (`cmd/*`, 450+ commands), vector edit mode, pointer tools, smart guides and snapping, component sync, library swapping, prototype runtime, Dev Mode code generation, and the panic guard. |
| **L6** | `crates/ui-egui` | Modern UI3 desktop interface: floating pill toolbar, left layers/assets panel, right Design/Prototype/Inspect panel, color picker, canvas render worker bridge, variable modal, font picker, accessibility bindings, and localhost control server. |
| **L6** | `crates/mcp` | Model Context Protocol (MCP) server implementation exposing FigCraft commands and scene graphs to external AI agents over stdio. |
| **L6** | `crates/collab-net` | WebSocket networking transport for live sessions: blocking `tungstenite`, TLS via `rustls`, connection limits, invite link handler, and session hub for multi-client collaboration. |
| **L7** | `apps/*` | Compiled deliverables: desktop app (`apps/figcraft`), CLI/MCP runner (`apps/figcraft-cli`), web browser client (`apps/figcraft-web`), and standalone collaboration server (`apps/figcraft-server`). |
| **Plugins** | `plugins/` | Isolated workspace containing the FigCraft Plugin Rust SDK and WASM example plugins compiled to `wasm32-unknown-unknown`. |

---

## 5. Concurrency, Multi-Threading & State Management

Unlike conventional single-threaded GUI tools, FigCraft implements a **hybrid multi-threaded model** that offloads heavy CPU rasterization, system font discovery, background saving, and real-time networking to dedicated worker threads, ensuring the UI remains fluid at 60+ FPS:

```
+-------------------------------------------------------------------------------------------+
| Main UI Thread (L7 apps/figcraft + L6 crates/ui-egui)                                     |
|  - Runs winit event loop, processes gestures & pointer tools                              |
|  - Dispatches commands via Session::execute (wrapped in catch_unwind panic guard)          |
|  - Draws immediate-mode UI3 chrome & canvas overlays (selection box, guides, handles)    |
+-------------------------------------------------------------------------------------------+
         │                           │                                │
         ▼ submits Job               ▼ spawns                         ▼ sends edits / presence
+--------------------+      +--------------------+      +-----------------------------------+
| "figcraft-render"  |      | "figcraft-save"    |      | "live-hub" / "live-conn" (L6)     |
| (Worker Thread)    |      | (Background Save)  |      |  - WebSocket TCP Listener/Writers |
|  - tiny-skia CPU   |      |  - Encodes ZIP     |      |  - figcraft_collab::Server CRDT   |
|  - Diff rendering  |      |    snapshots off   |      |  - Throttles presence (30 Hz)     |
|  - Coarse preview  |      |    the UI thread   |      |  - Wakes UI via NetLink Waker     |
+--------------------+      +--------------------+      +-----------------------------------+
```

### 5.1 The Retained CPU Canvas Render Worker (`crates/ui-egui/src/render_worker.rs`)
Canvas rendering does not block the UI thread. In `crates/ui-egui/src/render_worker.rs`:
1. **Dedicated Worker Thread Per Window:** Each window spawns a thread named `"figcraft-render"`.
2. **Two-Stage Progressive Refinement:**
   * When zooming, if full rendering takes longer than **12ms** (`PREVIEW_AFTER_MS`), the worker immediately computes a coarse preview rendered at **4× fewer pixels** (`PREVIEW_FACTOR = 4.0`), displays it instantly, and then refines it to full resolution.
3. **Retained Diff-Only Rendering (`figcraft_render::Retained`):**
   * On panning, the renderer reuses the previous pixmap and rasterizes *only* the newly uncovered strips.
   * On editing, the renderer recalculates *only* the bounding regions of layers that were marked touched (`Dirty`).
4. **Panic Isolation:** Worker rasterization is wrapped in `std::panic::catch_unwind`. If an invalid shape path causes an issue in `tiny-skia`, the worker catches the unwind, clears its pixmap cache, and keeps the application alive.

### 5.2 Immutable Persistent Scene Graph (`imbl`)
FigCraft avoids mutable object graphs or pointer spaghetti:
* The node hierarchy in `crates/doc` is stored inside `imbl::OrdMap<NodeId, Arc<Node>>`.
* `imbl` uses persistent Hash-Array Mapped Tries (HAMT) / B-Trees with structural sharing.
* Modifying a node creates a new `Arc<Node>` and updates the map in $O(\log N)$ time, leaving untouched nodes shared in memory.
* Taking an undo snapshot (`HistoryEntry`) requires only cloning an `Arc<Document>`—an instantaneous $O(1)$ pointer copy.

### 5.3 Partitioned Multiplayer Node ID Allocation (`IdAlloc`)
In single-user mode, node IDs are allocated sequentially. In live collaboration sessions, each participant allocates IDs within its own 32-bit partitioned space:
$$\text{NodeId} = (\text{client\_id} \ll 32) \mid \text{counter}$$
This guarantees that nodes created concurrently across different machines never collide, eliminating ID negotiation stalls.

### 5.4 Autosave Checkpoints & Crash Recovery (`apps/figcraft/src/recovery.rs`)
* **Periodic Autosave:** Every 30 minutes (`AUTOSAVE_SECS`), open documents automatically record a checkpoint in their version history.
* **Auto-Save Encoding Thread:** When changes occur, `apps/figcraft` waits 3 seconds of idle time (`AUTO_SAVE_SECS`) and spawns a background thread named `"figcraft-save"` to serialize and compress the document without stalling UI rendering.
* **Crash Recovery Directory:** Live recovery copies of unsaved files are written to `~/.config/figcraft/Recovery`. On launch, FigCraft detects any unsaved documents from an abnormal exit and reopens them automatically.

---

## 6. Component Creation & Rendering Pipeline

FigCraft follows an **immediate-mode presentation model** for the UI chrome combined with an **asynchronous retained CPU canvas** for design rendering:

```
FigApp::ui(ui)
   ├── logic(&ctx)                      <-- Autosaves, control queue, theme & UI scaling
   ├── tabs::tab_bar(...)               <-- Multi-document tab strip & window controls
   ├── egui::Panel::left("left_rail")   <-- 56px icon rail (Layers, Assets, History)
   ├── egui::Panel::left("left_panel")  <-- Layers tree view & Assets browser
   ├── egui::Panel::right("right_panel")<-- Design / Prototype / Dev Mode inspector
   └── egui::CentralPanel               <-- Central Canvas Area
         ├── canvas::show(...)          <-- Canvas viewport, pan/zoom interaction
         │     ├── Polls render_worker  <-- Retrieves latest completed Pixmap texture
         │     ├── Paints Canvas Texture<-- egui::Painter::image()
         │     └── Paints Overlays      <-- Smart guides, distance redlines, selection
         │                                  bounding boxes, rotation handles, cursors
         └── toolbar::show(...)         <-- Floating UI3 pill toolbar (Select, Shapes,
                                            Pen, Text, Components, Comments)
```

### 6.1 Canvas Overlays & Vector Edit Mode
While design contents are rendered by the CPU worker, all interactive chrome is painted by `egui::Painter` in real time:
* **Selection Outlines & Transform Handles:** 1px vector borders, corner resize squares, rotation curves, and squircle corner radius controls.
* **Smart Guides & Distance Measurements (`crates/engine/src/snap.rs`, `measure.rs`):** Holding `Alt/Option` displays distance measurements in points to adjacent layers with magenta alignment guides.
* **Vector Edit Mode (`crates/engine/src/vectoredit.rs`):** Displays Bézier control points, handle lines, bend points, and vertex anchors.
* **Multiplayer Cursors:** Other participants' cursor positions, name tags, and presence indicators are rendered at 60 FPS over the canvas.

---

## 7. Styling, Variables, Design Tokens & Features

### 7.1 Design Tokens & Theming (`crates/ui-egui/src/theme.rs`)
The interface aesthetics match modern Figma UI3 standards:
* **Token Palette (`theme::Tokens`):** Centralizes colors for canvas backgrounds, panel fills, border dividers, text hierarchy, hover states, and accent colors.
* **Rounded Floating Aesthetics:** Floating toolbars with pill contours (`corner_radius = 20.0`), subtle drop shadows, and dark/light mode switching (`theme::Tokens::dark()` / `theme::Tokens::light()`).

### 7.2 Figma-Style Variables Engine (`crates/doc/src/props.rs`)
FigCraft features full support for Figma Design Variables:
* **Variable Types:** Color, Float (numbers, dimensions, radius), String, and Boolean.
* **Variable Collections & Modes:** Multiple modes per collection (e.g. `Light` vs `Dark`, `Desktop` vs `Mobile`). Switching modes dynamically rebinds resolved values across the entire document.
* **Variable Scopes & Code Syntax:** Bindings to specific attributes (fill, stroke, corner radius, opacity) with custom code syntax definitions for Dev Mode exports.

### 7.3 Advanced Typography & Shaping (`crates/text`)
* **HarfBuzz Text Shaping:** Uses [`harfrust`] to perform OpenType font shaping, ligatures, kerning, and contextual alternates.
* **Variable Fonts:** Full support for variable font weight, width, slant, and optical sizing axes.
* **Bidirectional & Multi-Script Fallback:** Seamlessly handles mixed LTR and RTL scripts (Arabic, Hebrew) and loads regional CJK glyph variants dynamically via `crates/ui-egui/src/cjk.rs`.

---

## 8. Cargo Workspaces, Layering Enforcement & Build Impact

### 8.1 Workspace Structure & Dependency Management
The root `Cargo.toml` manages the workspace with Rust 2024 edition resolver behavior:

```toml
[workspace]
resolver = "3"
members = ["crates/*", "apps/*", "xtask"]
exclude = ["plugins"]

[workspace.package]
version = "0.10.0"
edition = "2024"
rust-version = "1.95"
license = "MIT OR Apache-2.0"
```

#### Key Architecture Rules Checked by `cargo xtask layers`:
1. **Strict Downward Dependency Rule:** Crates at layer $L$ may never depend on crates at layer $\ge L$.
2. **Permitted Intra-Layer Dependencies:** Intra-layer connections are forbidden by default, with only explicit whitelisted exceptions:
   * `figcraft-color` $\rightarrow$ `figcraft-geom` (in L0)
   * `figcraft-render` $\rightarrow$ `figcraft-format` (in L3)
   * `figcraft-mcp` $\rightarrow$ `figcraft-collab-net` (in L6)
3. **Network Isolation Rule:** The networking crates (`tungstenite`, `tokio`, `tokio-tungstenite`, `mio`, `socket2`) are **strictly prohibited below L6**. Only `crates/collab-net` may import networking libraries.
4. **UI Isolation Rule:** UI toolkit crates (`egui`, `eframe`, `winit`, `rfd`) are **strictly prohibited below L6**. Layers L0 through L5 are 100% headless.
5. **Separate Plugin Workspace:** WebAssembly plugins live in `plugins/` and are built for `wasm32-unknown-unknown` in an independent workspace via `cargo xtask plugins`.

---

## 9. Frontend Developer Guide: Components, Screens & Data Flow

### 9.1 The Immediate-Mode Mental Model
In `egui`, there is no persistent widget tree or Virtual DOM. Every frame (at 60 FPS), the UI function call stack executes from top to bottom, evaluates layout and inputs, and emits graphics primitives to the GPU.

```
Retained-Mode (React / Flutter / Qt):
State Changes -> Reconcile Virtual DOM -> Mutate Persistent Widget Objects -> Platform Draw

Immediate-Mode (egui / FigCraft):
Every Frame (60 FPS) -> Execute Function Call Tree -> Emit Vertex Meshes -> GPU
```

#### Where State Lives:
* **Document Model:** `app.session.doc()` (`crates/doc`). Read-only; mutated strictly through `app.run("command.id", json!({...}))`.
* **Session State:** `app.session` (`crates/engine`). Active tool, selection, undo/redo stacks.
* **Persistent UI State:** `app.ui` (`crates/ui-egui`). Panel widths, active tabs, dark mode, zoom levels.
* **Transient Widget State:** `ui.data_mut(|d| d.get_temp(id))` using `egui::Id`.

### 9.2 The "Thin UI" Principle & The Command Boundary
The most fundamental rule for frontend development in FigCraft:

> **The UI is thin: it reads engine state and acts exclusively through `app.run(command_id, json_params)`.**

```
+-------------------------------------------------------------+
|                        User Interface                       |
|   1. Inspects state: `app.session.ds()`                     |
|   2. Renders widgets & checks interactions: `resp.clicked()`|
|   3. Dispatches actions: `app.run("shape.rect", json!({...}))`|
+-------------------------------------------------------------+
                             |
                    app.run(id, params)
                             v
+-------------------------------------------------------------+
|                      Engine (Session)                       |
|   - Validates command parameters                            |
|   - Captures pre-command snapshot for Undo/Redo             |
|   - Runs command in `catch_unwind` (Never-Crash guarantee)  |
|   - Broadcasts changes to multiplayer peers                 |
|   - Updates document revision counter                       |
+-------------------------------------------------------------+
```

### 9.3 Practical Frontend Development Patterns

#### Pattern 1: Creating a Custom Design Widget (`crates/ui-egui/src/widgets.rs`)
Custom widgets use `ui.allocate_exact_size` and paint onto `ui.painter()`:

```rust
use egui::{Color32, Rect, Response, Sense, Stroke, Ui, pos2, vec2};
use crate::theme::Tokens;

/// Custom toggle icon button matching FigCraft UI3 aesthetic.
pub fn tool_button(ui: &mut Ui, icon: &str, active: bool, tooltip: &str) -> Response {
    let t = Tokens::get(ui.ctx());
    let size = vec2(28.0, 28.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());

    // Paint button background
    if active {
        ui.painter().rect_filled(rect, 6.0, t.accent.gamma_multiply(0.15));
        ui.painter().rect_stroke(rect, 6.0, Stroke::new(1.0, t.accent), egui::StrokeKind::Inside);
    } else if resp.hovered() {
        ui.painter().rect_filled(rect, 6.0, t.hover);
    }

    // Draw icon centered
    let font = egui::FontId::proportional(14.0);
    let color = if active { t.accent } else { t.text };
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, icon, font, color);

    resp.on_hover_text(tooltip)
}
```

#### Pattern 2: Adding a Property Inspector Section (`crates/ui-egui/src/right.rs`)
To add a new inspector section to the Design panel:

```rust
pub fn stroke_section(app: &mut FigApp, ui: &mut egui::Ui) {
    let Some(ds) = app.session.ds().ok() else { return };
    if ds.selection.is_empty() { return; }

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Stroke").strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("+").clicked() {
                // Dispatch command through the engine
                app.run("stroke.add", serde_json::json!({}));
            }
        });
    });

    // Render stroke list and properties...
}
```

---

## 10. Binary Assembly & Runtime Threading Architecture

### 10.1 Binary Assembly: What Level and Crate Binds Components Together?

In FigCraft's workspace, **Layer 7 (L7: Applications)** binds the underlying modular libraries into compiled executables:

| Binary / Target | Crate | Layer | Binds Together | Primary Role |
|---|---|---|---|---|
| `figcraft` | `apps/figcraft` | **L7** | `engine` (L5), `ui-egui` (L6), `collab-net` (L6), `eframe`, `wgpu`, `arboard`, `rfd`, `craft-window`, `fmv-macos-events` | **Desktop Native App** (macOS, Windows, Linux, BSD) |
| `figcraft-cli` | `apps/figcraft-cli` | **L7** | `engine` (L5), `doc` (L1), `render` (L3), `text` (L2), `svg` (L4), `format` (L3), `mcp` (L6) | **Headless CLI & MCP Server** (info, export, render, agent tools) |
| `figcraft-web` | `apps/figcraft-web` | **L7** | `engine` (L5), `ui-egui` (L6), `collab-net` (L6), `eframe::WebRunner` | **WebAssembly Browser Client** (WebGPU / WebGL2) |
| `figcraft-server` | `apps/figcraft-server` | **L7** | `collab` (L2), `collab-net` (L6), `format` (L3), `plugin` (L2) | **Standalone Headless Collaboration Server** (persists change logs & snapshots, serves WebSocket rooms) |

### 10.2 Runtime Threading: All Threads in the Compiled Application

The running desktop application manages multiple concurrent threads across L6 and L7:

| Thread Name | Crate | Layer | When Spawned | Role & Behavior |
|---|---|---|---|---|
| **Main / UI Thread** | `apps/figcraft` via `eframe` | **L7** | Application launch | Drives the OS event loop (`winit`), processes pointer/keyboard events, evaluates `eframe::App::logic` & `ui`, coordinates multi-window viewports, and renders GPU overlays. |
| **`"figcraft-render"`** | `crates/ui-egui` (`render_worker.rs`) | **L6** | Per window creation | Dedicated background worker that executes CPU scene rasterization via `tiny-skia` (`figcraft_render::Retained`). Performs coarse preview rendering and progressive refinement without stalling the UI. |
| **Font Scanner Thread** | `apps/figcraft` (`main.rs`) | **L7** | App startup | Spawns a background worker executing `figcraft_text::scan_system_fonts()` to index installed system fonts without blocking window launch. Wakes the UI loop upon completion via `ctx.request_repaint()`. |
| **`"figcraft-save"`** | `crates/ui-egui` (`lib.rs`) | **L6** | When auto-save triggers | Encodes document snapshots into `.figcraft` ZIP packages asynchronously. |
| **`"figcraft-control"`** | `apps/figcraft` (`control_server.rs`) | **L7** | Launched with `--control <port>` | TCP server listening on `127.0.0.1:<port>` for JSON-RPC automation commands. Spawns `"figcraft-control-conn"` per connection. |
| **`"figcraft-control-conn"`** | `apps/figcraft` (`control_server.rs`) | **L7** | Per incoming control TCP socket | Reads incoming JSON lines, queues them into `control_queue`, wakes the UI loop, awaits response, and replies over TCP. |
| **`"live-hub"`** | `crates/collab-net` (`hub.rs`) | **L6** | When hosting a live session | Runs the collaboration room coordinator owning `figcraft_collab::Server` per open document and routes CRDT messages. |
| **`"live-accept"`** | `crates/collab-net` (`hub.rs`) | **L6** | When hosting a live session | Accepts incoming WebSocket client TCP connections. |
| **`"live-conn"`** | `crates/collab-net` (`hub.rs`) | **L6** | Per active WebSocket client | Dedicated reader thread reading incoming binary messages from a client. |
| **`"live-write"`** | `crates/collab-net` (`hub.rs`) | **L6** | Per active WebSocket client | Dedicated writer thread sending queued binary updates and heartbeats to a client. |
| **`"live-guest"`** | `crates/collab-net` (`guest.rs`) | **L6** | When joining a remote session | Supervisor thread handling connection handshakes, reconnection backoff, and presence throttling. Spawns `"live-guest-read"` for inbound messages. |
| **GPU / Runtime Threads** | OS Graphics Drivers | **External** | Created by runtime | Metal, DirectX 12, or Vulkan driver threads for shader dispatch and AccessKit accessibility bus monitoring. |

#### WebAssembly (WASM) Threading Note:
When compiled for the web (`apps/figcraft-web` targeting `wasm32-unknown-unknown`), **worker threads are disabled or replaced by browser asynchronous tasks**. The canvas renders inline, and networking is driven directly by browser `web_sys::WebSocket` events on the browser event loop.
