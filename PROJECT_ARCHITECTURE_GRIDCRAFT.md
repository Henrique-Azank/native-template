# GridCraft — Project Architecture Overview

## 1. Executive Summary

GridCraft is an open-source, clean-room, Rust-native spreadsheet application engineered for Microsoft Excel parity and high-performance execution. It is architected around an **engine-first, command-driven design** where core spreadsheet data structures, recalculation graphs, and file parsers are strictly decoupled from the presentation layer.

GridCraft runs natively across macOS, Windows, Linux, and BSD via native executables, and in web browsers via WebAssembly (WASM).

---

## 2. Rust GUI Framework & Platform Shell

### 2.1 Framework Used
* **Primary GUI Toolkit:** [`egui`](https://github.com/emilk/egui) (version **0.36**), an immediate-mode GUI framework written in pure Rust.
* **Application Shell & Windowing Runner:** [`eframe`](https://github.com/emilk/egui/tree/master/crates/eframe) (version **0.36**), which wraps [`winit`](https://github.com/rust-windowing/winit) for window creation, event handling, and display scaling.
* **Rendering Hardware Backend:** [`wgpu`](https://wgpu.rs) (WebGPU abstraction layer) as the primary renderer across desktop platforms:
  * **macOS:** Metal backend.
  * **Windows:** DirectX 12 by default (with automated fallback to OpenGL/WARP).
  * **Linux / BSD:** Vulkan and OpenGL backends (with Wayland and X11 support).
  * **Web (WASM):** WebGPU with fallback to WebGL2 (Glow) via `trunk`.
* **Accessibility:** Integrated through [`accesskit`](https://accesskit.dev).

### 2.2 Framework Decoupling & Thin UI Principle
The UI crate (`crates/ui-egui`) sits at layer L6 and depends only on `crates/engine` and `crates/chart`. Layering is validated by `cargo xtask layers`. **Nothing below L6 depends on `egui`, `eframe`, `winit`, or `rfd`**, making the GUI frontend completely swappable (e.g., capable of being replaced with Slint, Iced, or native platform toolkits in the future).

---

## 3. Framework Modifications, Workarounds & Overrides

GridCraft does not simply use standard `egui` widgets. It overrides several aspects of `egui` and `eframe` to achieve desktop spreadsheet fidelity, crash resilience, and automation capabilities:

### 3.1 Pre-Validated Font Engine with Dynamic Fallback Chains
* **Crash Prevention with `skrifa`:** In standard `egui`, calling `Context::set_fonts` with a corrupt, truncated, or unreadable font file panics inside `epaint` during character map parsing. GridCraft pre-validates every system font file using Google's [`skrifa`](https://github.com/googlefonts/fontations) parser (`crates/ui-egui/src/theme.rs`, `font_parses()`) before passing it to `egui`. Any unparseable faces are dropped gracefully.
* **Background Font Discovery:** Rather than stalling startup to scan disk fonts, `crates/ui-egui/src/fonts.rs` runs a background thread to index installed system fonts. Once scanned, fonts are bound dynamically without freezing the frame loop.
* **CJK Han Face Order & Multi-Script Fallback:** Fallback glyph chains prioritize Han glyphs depending on the selected UI language (Japanese vs. Simplified Chinese vs. Korean) to preserve proper regional glyph variants, alongside dedicated fallbacks for Thai, Hebrew, and mathematical/UI symbols (`DejaVu Sans`, `Segoe UI Symbol`).
* **One-Frame Font Rebinding Gate:** When font definitions change, `SheetApp` skips painting for exactly one frame to allow `egui` to compile the new font atlas cleanly.

### 3.2 Viewport & Window Chrome Overrides
* **Custom Titlebar & Client-Area Extension:** On macOS, `viewport.with_fullsize_content_view(true).with_titlebar_shown(false).with_title_shown(false)` is enabled in `apps/gridcraft/src/main.rs`. GridCraft draws its own custom Excel-style title bar with a integrated Quick Access Toolbar (QAT), window title, and window control margins.
* **Native OS Menu Integration (`muda`):** On macOS, native AppKit menus are mounted using [`muda`](https://github.com/tauri-apps/muda), completely bypassing egui's in-window menus to provide a system menu bar (GridCraft, File, Edit, View, Insert, Format, Tools, Window, Help).
* **Apple Events Interception:** `apps/gridcraft/src/apple_events.rs` uses `fmv-macos-events` to intercept `kAEOpenDocuments` events (e.g., double-clicking a `.xlsx` in Finder or dragging onto the Dock) which `winit 0.30` does not natively dispatch.

### 3.3 Keyboard Zoom Hijack Suppression
* Standard `egui` intercepts `Ctrl/Cmd + Plus / Minus / 0` to scale the entire egui interface. This swallows standard spreadsheet shortcuts (such as inserting/deleting cells). GridCraft disables egui's default behavior via `ctx.options_mut(|o| o.zoom_with_keyboard = false)` and implements its own stepped zoom ladder (`WHEEL_ZOOM_LADDER`) with smooth exponential gliding.

### 3.4 GPU Backend Driver Safety on Windows
* In `apps/gridcraft/src/graphics.rs`, GridCraft probes DirectX 12 hardware adapters before `eframe` initializes. If a machine lacks DX12 GPU drivers, it falls back to OpenGL or WARP software rasterization, explicitly avoiding Vulkan by default to prevent crashes observed with legacy Intel UHD drivers (`igvk64.dll`).

### 3.5 Synthetic Input Injection for Agent Automation
* GridCraft overrides `eframe::App::raw_input_hook` to inject queued batches of synthetic `egui::Event` objects (`SheetApp.synthetic`). This enables end-to-end automated control via the localhost JSON-RPC control protocol (`apps/gridcraft/src/control_server.rs`) and headless UI testing (`egui_kittest`).

### 3.6 Cross-Platform Desktop Theme Synchronization
* Because Linux / X11 / Wayland often lack standard system theme notifications in `winit`, `crates/ui-egui/src/desktop_theme.rs` spawns a background thread that communicates with the XDG Desktop Portal via D-Bus (`zbus`) to reactively detect dark/light mode switches.

---

## 4. Application Core Segmentation & Layering

The codebase is organized into an 8-layer Cargo workspace enforced by `cargo xtask layers`. Lower layers cannot depend on higher layers.

```
+-------------------------------------------------------------------------+
| L7: Applications                                                        |
|     apps/gridcraft (Desktop) | apps/gridcraft-cli | apps/gridcraft-web  |
+-------------------------------------------------------------------------+
                                    |
+-------------------------------------------------------------------------+
| L6: Presentation & External Automation                                 |
|     crates/ui-egui (Excel-style frontend) | crates/mcp (Agent Protocol) |
+-------------------------------------------------------------------------+
                                    |
+-------------------------------------------------------------------------+
| L5: Application Engine & Facade                                         |
|     crates/engine (Session, Command Registry, Undo/Redo, IO, Clipboard) |
+-------------------------------------------------------------------------+
                                    |
+-------------------------------------------------------------------------+
| L3: Visualization                                                       |
|     crates/chart (Toolkit-agnostic Chart Layout & Rasterizer)           |
+-------------------------------------------------------------------------+
                                    |
+-------------------------------------------------------------------------+
| L2: Document Model, Recalculation & File Formats                       |
|     crates/model (COW Cell Store, Styles, Sheet, Workbook)             |
|     crates/calc  (Dependency Graph Recalculator, Dynamic Array Spill)   |
|     crates/xlsx  (Lenient XLSX, XLSB, CSV, TSV, ODS Reader & Writer)    |
|     crates/pdf   (PDF 1.7 Exporter & Print Generator)                   |
+-------------------------------------------------------------------------+
                                    |
+-------------------------------------------------------------------------+
| L1: Formulas & Standard Functions                                       |
|     crates/formula   (Pratt Parser, AST, Reference Adjustment)          |
|     crates/functions (400+ Pure Excel Functions, Dynamic Array Lifting) |
+-------------------------------------------------------------------------+
                                    |
+-------------------------------------------------------------------------+
| L0: Core Primitives & Number Formatting                                 |
|     crates/core   (CellRef, RangeRef, Value, CellError, Coercion)       |
|     crates/numfmt (Excel Number Format Codes, Sections, Conditions)     |
+-------------------------------------------------------------------------+
```

### Detailed Crate Responsibilities

| Layer | Crate | Primary Responsibilities |
|---|---|---|
| **L0** | `crates/core` | Cell coordinates (`CellRef`, `RangeRef`, A1 & R1C1 notation up to 1,048,576 rows × 16,384 columns), `Value` enum, `CellError`, Excel type coercion rules, 1900/1904 date serial conversions. |
| **L0** | `crates/numfmt` | Full Excel format code parsing and formatting (positive/negative/zero/text sections, bracketed conditions `[>100]`, colors `[Red]`, currency, fractions, scientific notation, elapsed time, `TEXT()` function). |
| **L1** | `crates/formula` | Tokenizer, Pratt recursive-descent parser, Abstract Syntax Tree (AST), formula printer (A1 and R1C1), and reference rewriting for copy/cut/fill and structural row/column insertions. |
| **L1** | `crates/functions` | Over 400 Excel-compatible pure functions (`SUM`, `XLOOKUP`, `INDEX`, `MATCH`, `LET`, `LAMBDA`, `FILTER`, `SORT`, etc.) with signature registries (`FnSpec`) and automatic array lifting. |
| **L2** | `crates/model` | Workbook and sheet state; interned style tables (fonts, borders, fills, alignments, number formats); conditional formatting rules; data validation; comments; shapes, drawings, and images; copy-on-write cell storage. |
| **L2** | `crates/calc` | Recalculation engine (`Calc`), directed dependency graph with range nodes, topological evaluation, dynamic array spilling (`#SPILL!`), and circular reference detection. |
| **L2** | `crates/xlsx` | High-performance OpenXML (XLSX), XLSB, CSV/TSV, and ODS import/export; DrawingML charts; operates on raw byte slices (no disk dependency, fully WASM-safe). |
| **L2** | `crates/pdf` | Minimalist PDF 1.7 engine generating vector drawing instructions and standard fonts for document printing and PDF export. |
| **L3** | `crates/chart` | Toolkit-independent chart layout and rendering primitives (bar, column, line, pie, scatter, area, sparklines). Generates abstract vector shapes without binding to any GUI framework. |
| **L5** | `crates/engine` | Central `Session` manager: open workbook lifecycle, active selection, undo/redo COW snapshots, system clipboard bridge, fill operations, sorting, filtering, PivotTable cache calculation, and the 300+ command catalog (`cmd/*`). |
| **L6** | `crates/ui-egui` | Excel-style desktop UI: virtualized spreadsheet grid, ribbon, formula bar, sheet tabs, backstage view, dialogs, task panes, and the control channel. |
| **L6** | `crates/mcp` | Model Context Protocol (MCP) server over stdio for AI agent automation (headless or bridged to a live window). |
| **L7** | `apps/*` | Binaries for desktop execution (`gridcraft`), headless command line / MCP tools (`gridcraft-cli`), and WebAssembly browser deployment (`gridcraft-web`). |

---

## 5. Concurrency, Multi-Threading & State Management

### 5.1 Recalculation Threading Model
* **Recalculation is Single-Threaded:** Formula evaluation in `crates/calc` currently runs on a **single thread**. Dependency graphs are resolved in topological order on the calling thread. Parallel recalculation of independent dependency subgraphs is planned on the roadmap (documented in `docs/hardware-parity.md`), but not yet implemented.
* **Targeted Background Worker Threads:** Multi-threading is intentionally applied to non-blocking I/O and operating system integration:
  1. **Font Discovery:** Background thread (`std::thread::spawn` in `crates/ui-egui/src/fonts.rs`) parses system fonts at launch.
  2. **Desktop Appearance Watcher:** Background thread in `crates/ui-egui/src/desktop_theme.rs` monitors OS dark/light mode via D-Bus on Linux.
  3. **Control Server:** Background TCP listener thread (`apps/gridcraft/src/control_server.rs`) accepts JSON-RPC agent commands and forwards them to the UI thread via `std::sync::mpsc::channel`.
  4. **Print Worker:** Desktop print jobs are spooled to worker threads via `craft-print` so print job submission does not stall UI rendering.

### 5.2 State Management & Sparse Copy-on-Write (COW) Storage
GridCraft achieves instant undo/redo and lightweight snapshots through structural sharing:

1. **64-Row Banded Cell Store (`crates/model/src/store.rs`):**
   * Cells are stored inside `CellStore` divided into horizontal bands of 64 rows:
     ```rust
     struct Band {
         rows: BTreeMap<u32, BTreeMap<u32, Cell>>,
     }
     pub struct CellStore {
         bands: BTreeMap<u32, Arc<Band>>,
         len: usize,
     }
     ```
   * Each 64-row band is wrapped in an `Arc`.
   * Taking a snapshot of the entire spreadsheet (for an undo point) is an $O(\text{bands})$ operation that only clones `Arc` pointers—no cell data is duplicated.
   * When a cell is modified, `Arc::make_mut` clones *only* that specific 64-row band, leaving all other bands untouched.

2. **Session Document State & History (`crates/engine/src/lib.rs`):**
   * Each open workbook is managed by a `DocState`:
     * `wb: Arc<Workbook>` (current state)
     * `saved: Arc<Workbook>` (state on disk; dirty checking is a single pointer comparison: `!Arc::ptr_eq(&self.wb, &self.saved)`)
     * `undo: Vec<HistoryEntry>` and `redo: Vec<HistoryEntry>` (snapshots containing `Arc<Workbook>` and `Selection`)
     * `calc: Calc` (dependency graph and cached cell values)

3. **Command-Driven Mutation:**
   * Every edit passes through `Session::execute(command_id, json_params)`.
   * Before running an undoable command, `Session` captures `before = (d.uid, d.wb.clone(), d.selection.clone())`.
   * If the workbook pointer changes after execution (`!Arc::ptr_eq(&d.wb, &wb)`), an entry is pushed to the undo stack (capped at 100 entries).

4. **Panic Isolation & Never-Crash Guarantee:**
   * In compliance with the repository's strict "Never crash" rule, `Session::execute` wraps command execution in:
     ```rust
     std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (spec.run)(self, &params)))
     ```
   * If any command encounters an unhandled panic, `Session` intercepts it, restores `d.wb` and `d.selection` from the `before` snapshot, rebuilds `d.calc`, and returns `Err(EngineError::Internal)`. The application remains alive, uncorrupted, and reports the error to the user via a modal dialog or toast.

---

## 6. Component Creation & Rendering Pipeline

GridCraft follows an **immediate-mode presentation model**. There are no persistent widget objects for rows, columns, or cells. The entire interface is evaluated frame-by-frame:

```
SheetApp::logic(ctx)
   ├── Poll control channel (JSON-RPC)
   ├── Poll async file imports & printing status
   ├── Check theme & interface zoom changes
   └── Verify font initialization

SheetApp::ui(ui)
   ├── ribbon::title_bar(...)
   ├── backstage::show(...) [if File tab open]
   ├── ribbon::show(...)
   ├── widgets::notice_bar(...)
   ├── formula_bar::show(...)
   ├── tabs::status_bar(...)
   ├── tabs::sheet_tabs(...)
   ├── pivot_pane::show(...) & panes::show(...)
   └── egui::CentralPanel -> grid::show(...)
         ├── Viewport geometry & scroll calculation (Geo)
         ├── Zoom gliding & wheel handling
         └── Custom Virtualized Grid Painting (paint_quadrant)
```

### 6.1 Virtualized Grid Rendering (`crates/ui-egui/src/grid.rs`)
The grid handles up to 1,048,576 rows and 16,384 columns at a steady 60+ FPS:
* **No Widget Allocation for Cells:** GridCraft does not use `egui::Grid` or `egui_extras::Table`. It directly allocates a single `Rect` covering the spreadsheet area and draws onto `egui::Painter`.
* **Viewport Culling (`Geo`):**
  * Grid geometry calculates which row range (`r0..=r1`) and column range (`c0..=c1`) intersect the visible bounding box.
  * Only cells within this visible range are retrieved from the `CellStore`.
* **Four-Quadrant Freeze Panes:**
  * When freeze panes are active, the screen is split into four distinct quadrants (Top-Left frozen, Top frozen header, Left frozen column, and the main scrollable quadrant).
  * `paint_quadrant(...)` is invoked for each active pane, clipping render boundaries cleanly.
* **Direct Primitive Drawing:**
  * **Gridlines:** Drawn with `p.line_segment(...)`.
  * **Cell Backgrounds:** Painted with `p.rect_filled(...)`.
  * **Gradient Conditional Data Bars:** Constructed as custom vertex meshes using `egui::Mesh` and `p.add(mesh)`.
  * **Borders:** Rendered with `paint_border(...)` supporting thin, medium, thick, dashed, dotted, and double lines.
  * **Text & Galleys:** Formatted values are laid out with `p.layout_no_wrap(...)` and cached.
  * **Text Spilling / Overflow:** If text exceeds cell width, GridCraft inspects adjacent cells; if they are unmerged and empty, text is allowed to spill horizontally across column boundaries (matching Excel's behavior).
  * **Selection Marquee & Fill Handle:** Painted with a 2px accent stroke, corner fill square, and animated marching-ants dashed strokes for clipboard selections.

### 6.2 Floating Canvas Objects
Charts, pictures, shapes, slicers, and freehand ink strokes are rendered in a separate overlay pass (`paint_overlays`):
* Floating objects are anchored to sheet coordinates (`CellRef` + offset `dx, dy`).
* The renderer translates sheet coordinates through `Geo` into screen coordinates, supporting interactive dragging, border selection, and corner resize handles.

---

## 7. Styling, Themes & Formatting Features

### 7.1 Application Theme Tokens (`crates/ui-egui/src/theme.rs`)
All UI colors and metrics are centralized in `theme::Tokens`:
* **Palettes:** Distinct light (`Tokens::light()`) and dark (`Tokens::dark()`) palettes matching modern spreadsheet aesthetics (fluent green accents, neutral gray ribbons, tinted selection fills).
* **Application to egui:** `theme::apply(ctx, dark)` configures `egui::Visuals`:
  * `v.panel_fill` and `v.window_fill` set to token backgrounds.
  * `v.selection.bg_fill` and `v.selection.stroke` configured with soft accent washes.
  * `v.window_corner_radius` and `v.menu_corner_radius` customized for modern rounded corners.
  * Text styles mapped to platform UI fonts (`ui_font(13.0)`, `ui_bold(18.0)`, etc.).

### 7.2 Spreadsheet Cell Styling Engine (`crates/model/src/style.rs`)
Cell styles in GridCraft mirror OpenXML specifications:
* **Interned Styles:** Styles are deduplicated and interned into a workbook-level style table (`Workbook.styles: BTreeMap<u32, Style>`). Cells reference styles by a compact `u32` index.
* **Style Properties:**
  * **Font:** Family name, point size, bold, italic, underline (single/double/accounting), strikethrough, color (`Color::Rgb` or `Color::Theme`).
  * **Fill:** Solid color, pattern types (`PatternType::Solid`, `LightGray`, `MediumGray`, etc.), or gradient fills.
  * **Borders:** Independent top, bottom, left, right, and diagonal borders with 14 distinct line styles (`Thin`, `Medium`, `Dashed`, `Double`, `Hair`, `MediumDashed`, etc.) and custom colors.
  * **Alignment:** Horizontal (`General`, `Left`, `Center`, `Right`, `Fill`, `Justify`), vertical (`Top`, `Center`, `Bottom`), text wrap, and indent level.
  * **Number Format:** Numeric format ID referring to built-in or custom format strings.

### 7.3 Number Formatting Engine (`crates/numfmt`)
GridCraft includes a standalone parser and renderer for Excel number format strings:
* Fully supports standard Excel format strings: `#,##0.00;[Red](#,##0.00);"zero";@`.
* Supports section splitting (positive, negative, zero, text), conditional brackets (`[>1000]`), date/time tokens (`YYYY-MM-DD HH:MM:SS`), fraction approximations (`# ??/??`), scientific notation (`0.00E+00`), and currency prefixes/suffixes.

### 7.4 Conditional Formatting (`crates/engine/src/cf.rs`)
* Evaluates dynamic rules against cell ranges:
  * **Cell Value Rules:** Between, equal, greater than, less than.
  * **Expression / Formula Rules:** Custom formulas evaluating to boolean results.
  * **Visual Formats:** Color scales (2-color and 3-color heatmaps), Data bars (gradient or solid fills with min/max bounds), and Icon sets (directional arrows, traffic lights, ratings).

### 7.5 Localization & Internationalization (`i18n`)
* **Interface Localization:** Translated strings for 8 languages: English (`en`), Spanish (`es`), Simplified Chinese (`zh`), Japanese (`ja`), Korean (`ko`), Russian (`ru`), Portuguese (`pt`), and Czech (`cs`).
* **Formula Localization:** Localized function names and list separators (e.g., German/French `;` vs. English `,` based on formula locale) handled seamlessly via `crates/ui-egui/src/formula_locale.rs`.


---

## 8. Cargo Workspaces, Layering Enforcement & Build Impact

### 8.1 Workspace Structure & Dependency Management

GridCraft is organized as a Cargo workspace with unified settings defined in the root `Cargo.toml`:

```toml
[workspace]
resolver = "3"
members = ["crates/*", "apps/*", "xtask"]
default-members = ["crates/*", "apps/gridcraft", "apps/gridcraft-cli", "xtask"]

[workspace.package]
version = "0.7.0"
edition = "2024"
rust-version = "1.90"
license = "MIT OR Apache-2.0"
repository = "https://github.com/storytold/gridcraft"
```

#### Key Workspace Mechanics:
1. **Modern Resolver (`resolver = "3"`):**
   * Uses Rust 2024 edition resolver behavior.
   * Prevents build dependencies and dev-dependencies from leaking unselected feature flags into normal library crates, ensuring clean target dependency graphs across platforms and WASM.
2. **Centralized Dependency Catalog (`[workspace.dependencies]`):**
   * All shared internal packages (`gridcraft-core`, `gridcraft-calc`, `gridcraft-model`, etc.) and external dependencies (`egui`, `eframe`, `serde`, `quick-xml`, `zip`, `image`) are locked to uniform versions across every crate.
   * Individual member crates simply specify `gridcraft-core = { workspace = true }` or `serde = { workspace = true }`.
   * **Result:** Eliminates multiple duplicated versions of fundamental libraries in the build tree, reducing binary size and compile times.
3. **Workspace-Wide Lint & Safety Policies:**
   * Global `[workspace.lints.rust]` enforces `unsafe_code = "forbid"` across all non-exempt crates.
   * Workspace-wide clippy rules deny panics (`clippy::unwrap_used`, `clippy::expect_used`, `clippy::panic`, `clippy::todo`, `clippy::unimplemented`, `clippy::unreachable`).
4. **Bifurcated Profile Optimization for Build Velocity:**
   * `[profile.dev.package."*"] opt-level = 2`: External dependencies (such as XML parsing, compression, cryptography, and image codecs) are compiled with high optimization even in debug mode, keeping developer test runs fast and responsive.
   * `[profile.dev] opt-level = 1`: Internal workspace crates compile quickly with light optimization for rapid edit-compile-test iteration.
   * `[profile.release]`: Configured with `opt-level = 3`, `lto = "thin"`, `codegen-units = 16`, and `debug = 0` for optimized production releases.

---

### 8.2 The Layering Architecture & Strict Rules

The workspace is not a loose set of libraries; it is a strictly structured hierarchy governed by `xtask/src/layers.rs`:

```
+-----------------------------------------------------------------------------------------+
| L7: Applications                                                                        |
|     apps/gridcraft (Desktop App) | apps/gridcraft-cli (CLI/MCP) | apps/gridcraft-web    |
+-----------------------------------------------------------------------------------------+
                                             |  depends only on L0-L6
+-----------------------------------------------------------------------------------------+
| L6: Presentation & Protocol Bridges                                                     |
|     crates/ui-egui (egui/eframe frontend) | crates/mcp (Model Context Protocol server)   |
+-----------------------------------------------------------------------------------------+
                                             |  depends only on L0-L5 (No UI below L6!)
+-----------------------------------------------------------------------------------------+
| L5: Engine Façade & Command Dispatch                                                    |
|     crates/engine (Session, Command Registry, Undo/Redo, IO, Clipboard, Pivot Tables)   |
+-----------------------------------------------------------------------------------------+
                                             |  depends only on L0-L3
+-----------------------------------------------------------------------------------------+
| L3: Visualization Primitives                                                           |
|     crates/chart (Toolkit-agnostic Chart Layout & Primitives)                           |
+-----------------------------------------------------------------------------------------+
                                             |  depends only on L0-L2
+-----------------------------------------------------------------------------------------+
| L2: Document Model, Recalc Engine & File Formats                                       |
|     crates/model (Workbook, Sheet, COW Bands)                                           |
|     crates/calc  (Dependency Graph & Recalculator)                                       |
|     crates/xlsx  (File import/export: XLSX, XLSB, CSV, ODS)                            |
|     crates/pdf   (Vector PDF generator)                                                 |
+-----------------------------------------------------------------------------------------+
                                             |  depends only on L0-L1
+-----------------------------------------------------------------------------------------+
| L1: Formulas & Function Implementations                                                 |
|     crates/formula   (Parser, AST, Reference Adjustment)                                |
|     crates/functions (400+ Pure Excel Functions, Array Lifting)                         |
+-----------------------------------------------------------------------------------------+
                                             |  depends only on L0
+-----------------------------------------------------------------------------------------+
| L0: Foundation Primitives                                                               |
|     crates/core   (CellRef, RangeRef, Value, CellError, Coercion, Serials)              |
|     crates/numfmt (Format String Parser, Condition & Color Sections)                    |
+-----------------------------------------------------------------------------------------+
```

#### Layer Invariant Rules (Checked by `cargo xtask layers`):
1. **Strict Downward Dependency Rule:**
   * A crate at layer $L$ may **only depend on crates at strictly lower layers** ($< L$).
   * Upward dependencies (e.g. `crates/model` depending on `crates/engine` or `crates/calc`) are forbidden and fail CI immediately.
2. **Explicit Intra-Layer Ordering (`INTRA_LAYER_ORDER`):**
   * Dependencies between crates in the *same* layer are forbidden by default.
   * Only specifically whitelisted linear chains are permitted:
     * `core` $\rightarrow$ `numfmt` (in L0)
     * `formula` $\rightarrow$ `functions` (in L1)
     * `model` $\rightarrow$ `calc` (in L2)
     * `model` $\rightarrow$ `xlsx` (in L2)
   * Note that `calc` and `xlsx` are peers: `xlsx` does not depend on `calc`, and `calc` does not depend on `xlsx`.
3. **Format Crate Isolation (`FORMAT_ALLOWED`):**
   * File format crates (`crates/xlsx`) are restricted to depending *only* on `core`, `formula`, and `model`.
   * They cannot depend on `calc`, `functions`, or `engine`. This guarantees that importing a workbook parses raw data and formulas without inadvertently triggering side-effects, full recalculations, or UI allocations.
4. **UI Toolkit Isolation (`UI_MIN_LAYER = 6`):**
   * Graphical toolkit dependencies (`egui*`, `eframe`, `winit`, `rfd`, `wgpu*`) are forbidden below layer L6.
   * Layers L0 through L5 are **100% headless, pure Rust libraries**.

---

### 8.3 How Workspaces and Layers Affect Building the Application

The layering architecture and workspace structure directly dictate how the application compiles, tests, and deploys:

#### 1. Maximum Parallel Compilation & Cycle-Free Directed Acyclic Graph (DAG)
* Because upward and circular dependencies are physically prevented by `cargo xtask layers`, `cargo` and `rustc` construct a clean, acyclic compilation DAG.
* Independent crates at the same layer compile concurrently across CPU cores:
  * In layer L0: `core`, `numfmt`, and `pdf` compile in parallel.
  * In layer L2: `calc` and `xlsx` compile concurrently once `model` is built.
* This avoids "bottleneck" crates that serialise the compilation of the entire repository.

#### 2. Fine-Grained Incremental Builds
* Changes are localized to the affected layer and its descendants:
  * Editing UI widgets, dialogs, or ribbon layouts in `crates/ui-egui` (L6) recompiles **only L6 and L7**. Layers L0–L5 (`core`, `formula`, `functions`, `model`, `calc`, `xlsx`, `chart`, `engine`) do **not** recompile.
  * Adjusting a formula function in `crates/functions` (L1) recompiles `calc`, `engine`, and UI, but leaves `core`, `numfmt`, `pdf`, `formula`, and `xlsx` untouched.

#### 3. Decoupled Binary Targets
The workspace separation allows building targeted executables without pulling unnecessary dependencies:
* **Headless CLI (`cargo build -p gridcraft-cli`):**
  * Compiles `crates/engine` and `crates/xlsx` without compiling `egui`, `eframe`, `wgpu`, or windowing systems.
  * Results in a lean, fast binary ideal for batch processing, automated XLSX/CSV conversions, formula evaluations, and server-side MCP agent tools.
* **Desktop App (`cargo build -p gridcraft`):**
  * Links the full stack including `crates/ui-egui`, native windowing (`winit`), native platform menus (`muda`), Apple Events (`fmv-macos-events`), and desktop printing (`craft-print`).
* **Web App (`apps/gridcraft-web`):**
  * Built using `trunk build apps/gridcraft-web/index.html`.
  * Pulls only WASM-compatible dependencies, substituting WebGPU/WebGL2 for native graphics.

#### 4. Automated WebAssembly (WASM) Verification (`cargo xtask wasm`)
* The layering system enables a formal gate for web support:
  * All crates from **L0 through L5** are guaranteed to be pure Rust, free of OS-specific threads, raw file system handles, or platform-native C libraries.
  * `cargo xtask wasm` runs `cargo check --target wasm32-unknown-unknown` against all L0–L5 crates plus `apps/gridcraft-web`.
  * This guarantees that core spreadsheet modeling, calculation, and XLSX parsing never regress web browser compatibility.

#### 5. Independent, Fast Subsystem Testing
* Tests can be executed on isolated subtrees of the workspace in milliseconds:
  * `cargo test -p gridcraft-calc` tests the recalculation engine without building the UI or graphics backend.
  * `cargo test -p gridcraft-functions` runs tests on 400+ mathematical and logical functions in isolation.
  * `cargo test -p gridcraft-xlsx` tests OpenXML round-trips against in-memory buffers.

#### 6. Continuous Integration Gate (`cargo xtask ci`)
Before any commit or PR is accepted, `cargo xtask ci` validates:
1. `cargo fmt --all -- --check` (code formatting)
2. `cargo clippy --workspace --all-targets -- -D warnings` (zero linter warnings)
3. `cargo test --workspace` (all unit and integration tests)
4. `cargo xtask assets` (attribution check for original assets)
5. `cargo xtask layers` (programmatic verification of dependency layering against `cargo metadata`)
6. `cargo xtask wasm` (WASM target build verification)
7. `cargo xtask parity --check` (catalog validation against Excel feature parity)


---

## 9. Frontend Developer Guide: Components, Screens & Data Flow

This section details how the "component tree" functions in GridCraft's immediate-mode architecture, and provides practical guides and patterns for frontend developers building new components, task panes, dialogs, and data-driven screens.

---

### 9.1 The Immediate-Mode "Component Tree" Mental Model

Unlike retained-mode GUI frameworks (such as React, Vue, Flutter, or Qt), `egui` does **not** construct or maintain a persistent in-memory tree of widget objects between frames:

```
Retained-Mode (React / Qt / Flutter):
State Changes -> Reconcile Virtual DOM -> Mutate Persistent Widget Objects -> Platform Draw

Immediate-Mode (egui / GridCraft):
Every Frame (60 FPS) -> Execute Call Graph of Render Functions -> Emit Draw Vertices -> GPU
```

#### Where is the "Component Tree"?
In GridCraft, the **"Component Tree" is the function call stack** executed every frame inside `SheetApp::ui`:

```
SheetApp::ui(&mut self, ui: &mut egui::Ui)
  ├── ribbon::title_bar(self, ui)
  ├── ribbon::show(self, ui)
  │     ├── ribbon::tab_home(self, ui)
  │     │     ├── ribbon::group("Font", ui)
  │     │     │     ├── widgets::toggle_button(ui, Icon::Bold, is_bold, "Bold")
  │     │     │     ├── widgets::toggle_button(ui, Icon::Italic, is_italic, "Italic")
  │     │     │     └── widgets::color_picker(ui, current_color, ...)
  │     │     └── ribbon::group("Alignment", ui)
  ├── formula_bar::show(self, ui)
  ├── tabs::status_bar(self, ui)
  ├── tabs::sheet_tabs(self, ui)
  ├── panes::show(self, ui)         <-- Task Panes (Comments, Watch, Inspector)
  ├── egui::CentralPanel            <-- Spreadsheet Grid & Overlays
  └── dialogs::show(self, ctx)      <-- Modal Dialogs (Format Cells, Sort, etc.)
```

#### How State Lives in an Immediate-Mode Component
Because components are functions that run and vanish every 16ms, state is categorized into four tiers:

| State Tier | Where it Lives | Examples | How Components Access It |
|---|---|---|---|
| **Document State** | `crates/model` via `app.session.active().wb` | Cells, formulas, styles, comments, sheets | Read-only inspection; mutated strictly via `app.run("cmd.id", params)` |
| **Session State** | `crates/engine` via `app.session` | Active cell, selection ranges, clipboard | Read via `app.session.active()`, updated via engine commands |
| **Persistent UI State** | `crates/ui-egui` in `app.ui` or `app.grid` | Dark mode, active ribbon tab, zoom level | Read/write through `&mut app.ui` or `&mut app.grid` |
| **Transient Widget State** | `egui` memory storage (`ctx.data_mut()`) | Text edit drafts, accordion open/closed, scroll offsets | `ui.data_mut(\|d\| d.get_temp(id))` using unique `egui::Id`s |

---

### 9.2 The "Thin UI" Principle & The Command Boundary

The most critical architectural rule for frontend developers in GridCraft is:

> **The UI is thin: it reads engine state and acts exclusively through `app.run(command_id, json_params)`.**

```
+-------------------------------------------------------------+
|                        User Interface                       |
|   1. Inspects state: `app.session.active()`                 |
|   2. Renders widgets & checks interactions: `resp.clicked()`|
|   3. Dispatches actions: `app.run("cell.set", json!({...}))`|
+-------------------------------------------------------------+
                             |
                   app.run(id, params)
                             v
+-------------------------------------------------------------+
|                      Engine (Session)                       |
|   - Validates command parameters                            |
|   - Captures pre-command snapshot for Undo/Redo             |
|   - Runs command in `catch_unwind` (Never-Crash guarantee)  |
|   - Evaluates dependency graph recalculation                |
|   - Updates revision counter & triggers UI requests         |
+-------------------------------------------------------------+
```

**Why this rule is mandatory:**
1. **Free Undo/Redo:** Direct mutations bypass the history snapshotting mechanism; commands automatically gain undo/redo tracking.
2. **Agent Automation:** Every action built this way is immediately reachable by automated AI agents via MCP (`gridcraft-cli mcp`) and the control protocol (`--control 7979`).
3. **Never-Crash Safety:** `Session::execute` catches panics and rolls back corrupted states. Direct mutations lack this isolation.

---

### 9.3 Concrete Examples for Frontend Developers

#### Pattern 1: Creating a Custom UI Widget (`crates/ui-egui/src/widgets.rs`)

When standard `egui` widgets (buttons, checkboxes) don't match the required spreadsheet aesthetic, developers build custom immediate-mode widgets using `ui.allocate_exact_size` and `ui.painter()`.

Here is an example of a custom **Badge / Tag Button**:

```rust
use egui::{Color32, Rect, Response, Sense, Ui, vec2, pos2, Stroke, StrokeKind};
use crate::theme::{self, Tokens};

/// Custom interactive status badge widget.
pub fn status_badge(ui: &mut Ui, label: &str, active: bool, color: Color32) -> Response {
    let t = Tokens::get(ui.ctx());
    let font = theme::ui_font(11.0);
    
    // 1. Calculate text layout to determine required size
    let text_galley = ui.painter().layout_no_wrap(label.to_string(), font.clone(), t.text);
    let badge_size = vec2(text_galley.size().x + 16.0, 20.0);

    // 2. Allocate interactive space on the layout
    let (rect, resp) = ui.allocate_exact_size(badge_size, Sense::click());

    // 3. Paint visuals based on interaction states
    if resp.is_pointer_button_down_on() {
        ui.painter().rect_filled(rect, 10.0, t.pressed);
    } else if resp.hovered() {
        ui.painter().rect_filled(rect, 10.0, t.hover);
    } else if active {
        ui.painter().rect_filled(rect, 10.0, color.gamma_multiply(0.2));
        ui.painter().rect_stroke(rect, 10.0, Stroke::new(1.0, color), StrokeKind::Inside);
    } else {
        ui.painter().rect_filled(rect, 10.0, t.window);
    }

    // 4. Draw indicator dot and label
    let dot_center = pos2(rect.left() + 7.0, rect.center().y);
    ui.painter().circle_filled(dot_center, 3.0, color);
    ui.painter().galley(pos2(rect.left() + 13.0, rect.center().y - text_galley.size().y / 2.0), text_galley, t.text);

    resp
}
```

#### Pattern 2: Managing Draft / Uncommitted Widget State (`egui::Id`)

Spreadsheet inputs frequently require holding an uncommitted text draft while typing, only updating the engine when focus is lost or Enter is pressed. Use egui's temporary data store:

```rust
pub fn cell_tag_input(ui: &mut Ui, id: egui::Id, current_tag: &str) -> Option<String> {
    let draft_id = id.with("draft_text");
    
    // Retrieve cached draft or fallback to the current document value
    let mut draft = ui.data_mut(|d| d.get_temp::<String>(draft_id))
        .unwrap_or_else(|| current_tag.to_string());

    let response = ui.add(egui::TextEdit::singleline(&mut draft).desired_width(120.0));

    if response.has_focus() {
        ui.data_mut(|d| d.insert_temp(draft_id, draft));
        None
    } else if response.lost_focus() {
        ui.data_mut(|d| d.remove::<String>(draft_id));
        // Return Some(draft) only if the user didn't hit Escape
        if !ui.input(|i| i.key_pressed(egui::Key::Escape)) && draft != current_tag {
            Some(draft)
        } else {
            None
        }
    } else {
        None
    }
}
```

#### Pattern 3: Adding a New Task Pane / Side Panel (`crates/ui-egui/src/panes.rs`)

To create a new dockable side panel (e.g., an **Audit Log Pane**):

```rust
// In crates/ui-egui/src/panes.rs:

pub fn show(app: &mut SheetApp, ui: &mut Ui) {
    // 1. Check if our pane is active
    let Some(pane) = &app.grid.pane else { return };
    if pane != "auditLog" { return; }

    let t = Tokens::get(ui.ctx());
    let max_w = (ui.available_width() / 2.0).max(320.0);

    // 2. Open an egui side panel on the right
    egui::Panel::right("audit_pane")
        .default_size(320.0)
        .max_size(max_w)
        .resizable(true)
        .frame(egui::Frame::NONE.fill(t.ribbon).inner_margin(12))
        .show(ui, |ui| {
            // Header with close button
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Audit Log").font(theme::ui_bold(15.0)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✕").clicked() {
                        app.grid.pane = None;
                    }
                });
            });
            ui.separator();

            // 3. Read data from the engine
            let journal = &app.session.journal;
            if journal.is_empty() {
                ui.label(egui::RichText::new("No actions recorded yet.").italics());
            }

            // 4. Render scrollable content list
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (idx, (cmd, params)) in journal.iter().enumerate().rev() {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.strong(format!("#{idx} {cmd}"));
                            if ui.small_button("Revert").clicked() {
                                // 5. Trigger engine command on interaction
                                app.run_or_alert("edit.undo", json!({}));
                            }
                        });
                        ui.label(egui::RichText::new(params.to_string()).small().monospace());
                    });
                    ui.add_space(4.0);
                }
            });
        });
}
```

#### Pattern 4: Adding a New Modal Dialog (`crates/ui-egui/src/dialogs.rs`)

Modal dialogs can be created either as declarative forms or custom layout windows:

```rust
// In crates/ui-egui/src/dialogs.rs:

// Method A: Declarative Simple Form (Auto-generates UI fields and executes command on OK)
Dialog::form(
    "scaleValues",
    "Scale Values",
    "math.multiplyRange", // Target engine command
    vec![
        Field::Number { key: "factor", label: "Multiplication Factor:" },
        Field::Check { key: "round", label: "Round to nearest integer" },
    ],
    json!({"factor": 1.5, "round": false}),
)

// Method B: Custom Window Dialog
fn custom_export_dialog(app: &mut SheetApp, ui: &mut egui::Ui, d: &mut Dialog, confirm: &mut bool) {
    ui.label("Configure customized export settings:");
    
    let mut selected_fmt = d.values.get("format").and_then(Json::as_str).unwrap_or("csv").to_string();
    ui.horizontal(|ui| {
        ui.radio_value(&mut selected_fmt, "csv".into(), "CSV");
        ui.radio_value(&mut selected_fmt, "json".into(), "JSON");
        ui.radio_value(&mut selected_fmt, "pdf".into(), "PDF");
    });
    d.values.insert("format".into(), json!(selected_fmt));

    ui.add_space(8.0);
    
    // Standard OK / Cancel row
    ui.horizontal(|ui| {
        if ui.button("Export").clicked() {
            *confirm = true; // Triggers d.command execution
        }
        if ui.button("Cancel").clicked() {
            app.dialog = None;
        }
    });
}
```

---

### 9.4 Full-Stack Development Workflow: Adding New Features

When building a new capability from model to UI, follow this 5-step lifecycle:

```
[1. crates/model]    --> Define data structure on Sheet / Workbook
[2. crates/engine]   --> Implement CommandSpec in cmd/* (Undoable, Journaled)
[3. crates/ui-egui]  --> Add presentation logic in a dedicated module
[4. crates/ui-egui]  --> Mount render call inside SheetApp::ui
[5. crates/ui-egui]  --> Add Ribbon button / Keytip in ribbon.rs
```

#### Concrete Walkthrough: Adding a "Cell Bookmarks" Feature

#### Step 1: Model Storage (`crates/model/src/sheet.rs`)
Add bookmark storage to the worksheet data structure:
```rust
pub struct Sheet {
    // ... existing fields ...
    pub bookmarks: std::collections::BTreeMap<String, CellRef>,
}
```

#### Step 2: Engine Command (`crates/engine/src/cmd/bookmarks.rs`)
Implement the command to mutate bookmarks safely:
```rust
use crate::cmd::{CommandSpec, Result};
use crate::Session;
use serde_json::Value as Json;

pub fn add_bookmark_spec() -> CommandSpec {
    CommandSpec {
        id: "bookmark.add",
        label: "Add Bookmark…",
        params: &["name", "cell"],
        undoable: true,
        journal: true,
        enabled: Box::new(|session| session.active().is_some().then_some(()).ok_or("no document")),
        run: Box::new(|session, params| {
            let name = params["name"].as_str().ok_or("missing bookmark name")?;
            let cell = params["cell"].as_str().ok_or("missing cell")?;
            
            let doc = session.doc_mut()?;
            let cell_ref = gridcraft_core::CellRef::parse(cell).ok_or("invalid cell")?;
            
            // Copy-On-Write mutation
            let sheet = std::sync::Arc::make_mut(&mut std::sync::Arc::make_mut(&mut doc.wb).sheets[doc.wb.active_sheet]);
            sheet.bookmarks.insert(name.to_string(), cell_ref);
            
            Ok(serde_json::json!({"name": name, "cell": cell}))
        }),
    }
}
```

#### Step 3: UI Component (`crates/ui-egui/src/bookmarks_pane.rs`)
Create the presentation module:
```rust
use egui::Ui;
use serde_json::json;
use crate::SheetApp;

pub fn show(app: &mut SheetApp, ui: &mut Ui) {
    let Some(doc) = app.session.active() else { return };
    let Some(sheet) = doc.wb.active() else { return };

    ui.heading("Bookmarks");
    ui.separator();

    for (name, cell) in &sheet.bookmarks {
        ui.horizontal(|ui| {
            if ui.link(name).clicked() {
                // Jump selection to bookmarked cell
                let _ = app.run("selection.set", json!({"cell": cell.a1()}));
                app.grid.ensure_visible = true;
            }
            ui.label(cell.a1());
            if ui.small_button("✕").clicked() {
                let _ = app.run("bookmark.delete", json!({"name": name}));
            }
        });
    }
}
```

#### Step 4: Mount into Frame Loop (`crates/ui-egui/src/lib.rs`)
Mount the new component inside `SheetApp::ui`:
```rust
// Inside SheetApp::ui(&mut self, ui: &mut egui::Ui):
if self.grid.pane.as_deref() == Some("bookmarks") {
    egui::Panel::right("bookmarks_pane").show(ui, |ui| {
        bookmarks_pane::show(self, ui);
    });
}
```

#### Step 5: Wire the Ribbon Trigger (`crates/ui-egui/src/ribbon.rs`)
Add a button to the View tab:
```rust
if widgets::small_button(ui, Icon::Bookmark, "Bookmarks", "Show Bookmarks Pane", false).clicked() {
    app.grid.pane = Some("bookmarks".into());
}
```

---

### 9.5 Testing Frontend Components

GridCraft uses [`egui_kittest`](https://github.com/emilk/egui/tree/master/crates/egui_kittest) for deterministic, headless snapshot testing of UI components without needing a real window:

```rust
#[test]
fn test_bookmarks_pane_renders() {
    let session = gridcraft_engine::Session::new();
    let mut app = SheetApp::new(session, Services::default());
    app.grid.pane = Some("bookmarks".into());

    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::vec2(800.0, 600.0))
        .build_ui(|ui| {
            app.ui(ui);
        });

    harness.run(); // Simulates frame passes
    // Can assert harness state, inspect widget tree, or match image snapshots
}
```


---

## 10. Binary Assembly & Runtime Threading Architecture

This section explains which level and crates bind the components together into compiled executable binaries, and catalogues every thread running in the compiled application, detailing which crate and layer instantiates it.

### 10.1 Binary Assembly: What Level and Crate Binds Components Together?

In GridCraft's workspace, **Layer 7 (L7: Applications)** is the level that binds all underlying libraries together to produce executable binaries. 

There are three distinct application crates in `apps/`:

| Binary / Target | Crate | Layer | Binds Together | Primary Role |
|---|---|---|---|---|
| `gridcraft` | `apps/gridcraft` | **L7** | `engine` (L5), `ui-egui` (L6), `eframe`, `wgpu`, `arboard`, `rfd`, `muda`, `fmv-macos-events`, `craft-print` | **Desktop Native App** (macOS, Windows, Linux, BSD) |
| `gridcraft-cli` | `apps/gridcraft-cli` | **L7** | `engine` (L5), `mcp` (L6), `calc` (L2), `xlsx` (L2) | **Headless CLI & MCP Server** (eval, convert, run, test) |
| `gridcraft-web` | `apps/gridcraft-web` | **L7** | `engine` (L5), `ui-egui` (L6), `eframe::WebRunner` | **WebAssembly (WASM) Bundle** running in web browsers |

#### Deep Dive: How `apps/gridcraft` Assembles the Desktop Binary

The desktop executable is assembled in `apps/gridcraft/src/main.rs`. It serves as the master glue between the headless engine, the egui presentation layer, and the operating system:

1. **Logging & Panic Sinks (`apps/gridcraft/src/logging.rs`):**
   * Configures process-wide logging to both standard error and rolling log files (`ui.json` directory).
   * Installs a panic hook that logs detailed backtraces before unwinding.
2. **Engine Instantiation (`crates/engine`):**
   * Creates the `Session`, initializes a blank `Workbook` or loads specified files / sample templates (`--sample sales`).
3. **Platform Services Injection (`apps/gridcraft/src/main.rs::services()`):**
   * Plugs operating system capabilities into `gridcraft_ui_egui::Services`:
     * **Clipboard:** Wraps `arboard::Clipboard` for reading and writing HTML and plain text flavours.
     * **File Pickers:** Binds `rfd` (Rust File Dialogs) for native open/save dialogs.
     * **Printing:** Binds `apps/gridcraft/src/print.rs` which interfaces with `craft-print` for printer discovery and PDF job spooling.
     * **URL Opening:** Uses system launchers (`open` on macOS, `explorer` on Windows, `xdg-open` on Linux).
4. **App Composition (`SheetApp` $\rightarrow$ `struct App`):**
   * Constructs `SheetApp::new(session, services)` from `gridcraft-ui-egui`.
   * Wraps it in `struct App(SheetApp, Option<NativeMenu>, fmv_macos_events::Inbox)`.
   * Implements `eframe::App` lifecycle methods:
     * `fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame)`: polls native macOS menus (`muda`), Apple Events, control channel requests, and print feedback.
     * `fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput)`: handles drag-and-drop file openings and injects synthetic agent events.
     * `fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame)`: drives the entire immediate-mode render loop (`SheetApp::ui`).
     * `fn on_exit(&mut self)`: flushes pending print spoolers and persists UI preferences (`ui.json` and `prefs.json`).
5. **Windowing & Graphics Initialization (`eframe::run_native`):**
   * Configures `egui::ViewportBuilder` (window title, size, minimum dimensions, app id, drag-and-drop, and macOS titlebar concealment).
   * In `apps/gridcraft/src/graphics.rs`, runs GPU backend configuration on Windows (favoring DirectX 12, falling back to OpenGL/WARP, avoiding Vulkan driver crashes).
   * Hands `eframe::NativeOptions` and `Box::new(|cc| ...)` to `eframe::run_native()`, which hands control over to `winit`'s event loop and `wgpu`'s rendering surface.

---

### 10.2 Runtime Threading: All Threads in the Compiled Application

GridCraft is intentionally architected around a **single main thread for UI, event handling, and calculation**, augmented by dedicated background worker threads for non-blocking I/O and operating system integration.

Here is the complete inventory of all threads running in the compiled desktop application:

| Thread Name | Crate | Layer | When Spawned | Role & Behavior |
|---|---|---|---|---|
| **Main / UI Thread** | `apps/gridcraft` via `eframe` | **L7** | Application launch | Runs the OS event loop (`winit`), processes keyboard/mouse input, evaluates `eframe::App::logic` & `ui`, drives `wgpu` frame rendering, and executes spreadsheet recalculation (`crates/calc`). |
| **`"font-scan"`** | `crates/ui-egui` (`fonts.rs`) | **L6** | First context setup (`setup_context`) | Scans platform font folders (`/System/Library/Fonts`, `C:\Windows\Fonts`, `/usr/share/fonts`, user font dirs) for font families, styles, and weights using `skrifa`. Terminates once indexing is complete and populates `OnceLock<Vec<Face>>`. |
| **`"gridcraft-desktop-theme"`** | `crates/ui-egui` (`desktop_theme.rs`) | **L6** | App startup on **Linux only** | Queries the XDG Desktop Portal over D-Bus (`zbus`) every 4 seconds to detect OS light/dark mode changes. Sends update signals to the UI thread via `std::sync::mpsc::channel`. |
| **`"control"`** | `apps/gridcraft` (`control_server.rs`) | **L7** | Launched with `--control <port>` or `GRIDCRAFT_CONTROL_PORT` | Listens on a localhost TCP socket (`127.0.0.1:<port>`). When a client connects, accepts the stream and spawns a dedicated `"control-conn"` worker thread. |
| **`"control-conn"`** | `apps/gridcraft` (`control_server.rs`) | **L7** | Per incoming TCP connection to the control port | Reads JSON lines from the client, converts them to `ControlRequest`, queues them on an MPSC channel, calls `ctx.request_repaint()` to wake the UI loop, awaits the response, and writes the JSON reply back over TCP. |
| **`"print-discovery"`** | `apps/gridcraft` (`print.rs`) | **L7** | When user enters Print page or triggers printer discovery | Asynchronously invokes `craft_print::printers()` off the UI thread to query the OS print subsystem (CUPS on macOS/Linux, WinSpool on Windows). Sends discovered printer lists back via MPSC channel. |
| **`"print-submit"`** | `apps/gridcraft` (`print.rs`) | **L7** | When a print job is submitted | Spools the generated PDF bytes and settings to the operating system's native spooler asynchronously. Tracked in a global `JOBS` mutex so the application shutdown waits for print jobs to flush. |
| **Driver / GPU Helper Threads** | OS / Hardware Drivers (`wgpu`, `winit`, graphics driver) | **External** | Created by graphics runtime | Background driver threads managed by Metal, DirectX 12, or Vulkan for shader compilation, GPU submission synchronization, and OS accessibility bus monitoring (AccessKit). |

#### WebAssembly (WASM) Threading Note:
When compiled for the web (`apps/gridcraft-web` running under `wasm32-unknown-unknown`), **all worker threads are disabled or no-ops**. The entire web application runs strictly on the browser's single JavaScript/WASM event thread.
