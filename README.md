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
9. **Cross-Platform Packaging & Installers:** Zero-dependency native packaging scripts for macOS (`.dmg` + `.app`), Windows (`.msi` wizard via WiX v5 + `-portable.zip`), Linux (`.deb`, `.rpm`, `.AppImage`, `.tar.gz`), FreeBSD (`.tar.gz`), and Web (`.zip`).

---

## Workspace Structure

```
native-template/
├── Cargo.toml                       # Workspace definition (resolver="3", unified version, deps & lints)
├── assets/
│   └── app-icon/
│       └── template-app.svg         # Single 512x512 master vector icon (generates .icns, .ico, PNGs)
├── xtask/                           # Automation, CI quality gates & release helpers
│   └── src/
│       ├── main.rs                  # xtask CLI: ci, layers, wasm, version, ico
│       ├── layers.rs                # Programmatic L0-L7 layer verification
│       ├── version.rs               # Single-source version reader & setter
│       └── ico.rs                   # Pure-Rust multi-resolution Windows .ico packer
├── crates/
│   ├── core/                        # [L0] Core primitives, IDs, scalar values, error types
│   ├── model/                       # [L1] Document Object Model with persistent COW data structures (imbl/Arc)
│   ├── engine/                      # [L5] Central Session, CommandSpec registry (cmd/*), undo/redo, prefs
│   ├── net/                         # [L6] Async HTTP client, two-tier cache (memory + disk), stale-while-revalidate
│   ├── ui-egui/                     # [L6] Presentation: React-like components, pages, router, multi-window manager
│   └── mcp/                         # [L6] Model Context Protocol server for AI agent automation
├── apps/
│   ├── desktop/                     # [L7] Native desktop executable (macOS, Windows, Linux, FreeBSD)
│   ├── cli/                         # [L7] Headless command line interface & MCP runner (eval, batch commands)
│   └── web/                         # [L7] WebAssembly browser client (Trunk / WebGPU / WebGL2)
└── packaging/                       # OS-native installer builders & distribution manifests
    ├── env.sh                       # Shared environment variables, version resolver & SHA-256 helpers
    ├── icons.sh                     # Regenerates .icns, .ico, and hicolor PNGs from master SVG
    ├── macos/                       # macOS .app bundle, codesign, notarytool & .dmg builder
    ├── windows/                     # Windows WiX v5 .msi wizard, Authenticode sign.ps1 & portable .zip
    ├── linux/                       # Linux .deb, .rpm (nFPM), .AppImage (appimagetool) & .tar.gz
    ├── freebsd/                     # FreeBSD .tar.gz with /usr/local installer script
    └── web/                         # Trunk WASM release bundler with _headers & .htaccess
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
# Print CLI version + embedded git SHA and build date
cargo run -p template-cli -- --version

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

## Cross-Platform Packaging & Distribution

Every release artifact is generated from `packaging/<os>/package.{sh,ps1}` into `dist/` alongside a `.sha256` checksum sidecar.

### Version Management & Icons

The workspace version lives in **one place**: `[workspace.package] version` in the root `Cargo.toml`.

```bash
# Print current workspace version
cargo xtask version

# Atomically bump version across the workspace
cargo xtask version set 0.2.0

# Regenerate macOS .icns, Windows .ico, and Linux hicolor PNGs from assets/app-icon/template-app.svg
./packaging/icons.sh
```

---

### Distribution Matrix & Build Commands by OS

| Target OS | Script | Generated Artifacts in `dist/` | Host Prerequisites |
| :--- | :--- | :--- | :--- |
| **macOS** | `./packaging/macos/package.sh [universal\|arm64\|x86_64]` | `TemplateApp-<ver>-macos-<arch>.dmg`<br>`template-cli-<ver>-macos-<arch>.zip` | Xcode Command Line Tools (`lipo`, `codesign`, `hdiutil`, `xcrun notarytool`) |
| **Windows** | `pwsh packaging/windows/package.ps1 -Arch [x64\|x86\|arm64]` | `TemplateApp-<ver>-windows-<arch>.msi`<br>`TemplateApp-<ver>-windows-<arch>-portable.zip` | PowerShell 7+, MSVC toolchain, WiX Toolset v5 (`dotnet tool install --global wix` + `wix extension add -g WixToolset.UI.wixext/5.0.2`) |
| **Linux** | `./packaging/linux/package.sh [--skip-build]` | `template-app-<ver>-linux-<arch>.tar.gz`<br>`template-app_<ver>_<deb-arch>.deb`<br>`template-app-<ver>-1.<rpm-arch>.rpm`<br>`TemplateApp-<ver>-<arch>.AppImage` | `strip`, `readelf`, [`nfpm`](https://nfpm.goreleaser.com/) (for `.deb`/`.rpm`), [`appimagetool`](https://github.com/AppImage/appimagetool) (for `.AppImage`) |
| **FreeBSD** | `./packaging/freebsd/package.sh` | `template-app-<ver>-freebsd-<arch>.tar.gz` | FreeBSD host with Rust toolchain |
| **Web (WASM)** | `./packaging/web/package.sh` | `template-web-<ver>.zip` | `rustup target add wasm32-unknown-unknown`, `cargo install --locked trunk wasm-bindgen-cli` |

---

### Environment Variables Reference

#### 1. Global Build & Packaging Variables (`packaging/env.sh`)

| Variable | Required | Default | Description |
| :--- | :---: | :--- | :--- |
| `TEMPLATE_VERSION` | No | `[workspace.package] version` in `Cargo.toml` | Overrides the release version string embedded into filenames and manifests. |
| `DIST` | No | `<repo>/dist` | Output directory where packaged installers and `.sha256` files are written. |
| `CARGO_TARGET_DIR` | No | `<repo>/target` | Cargo build target directory used when compiling or locating pre-built binaries. |
| `TEMPLATE_BUILD_SHA` | No | `git rev-parse --short=9 HEAD` (`unknown` if not git) | Short commit hash baked into binaries via `option_env!("TEMPLATE_BUILD_SHA")`. |
| `TEMPLATE_BUILD_DATE` | No | Current UTC date (`YYYY-MM-DD`) | Build date baked into binaries and AppStream release metadata. |

#### 2. macOS Signing & Notarization Variables (`packaging/macos/`)

When `MACOS_SIGN_IDENTITY` is unset, `packaging/macos/package.sh` performs an **ad-hoc sign (`-`)** suitable for local testing. Set the variables below in CI for Developer ID distribution and Apple Gatekeeper notarization:

| Variable | Required | Default | Description |
| :--- | :---: | :--- | :--- |
| `APP_BUNDLE_ID` | No | `com.example.templateapp` | Reverse-DNS macOS bundle identifier written into `Info.plist`. |
| `MACOS_SIGN_IDENTITY` | For Release | `-` (ad-hoc) | Code signing identity, e.g. `"Developer ID Application: Example Corp (TEAMID)"`. Automatically exported if you source `packaging/macos/import-cert.sh`. |
| `MACOS_KEYCHAIN` | No | Default login keychain | Path to the temporary keychain holding the signing certificate (set by `import-cert.sh`). |
| `APPLE_CERTIFICATE` | For CI Cert Import | *None* | Base64-encoded `.p12` Developer ID Application certificate (`base64 -i cert.p12 \| pbcopy`). Used by `source packaging/macos/import-cert.sh`. |
| `APPLE_CERTIFICATE_PASSWORD` | For CI Cert Import | *None* | Passphrase used to decrypt `APPLE_CERTIFICATE`. |
| `KEYCHAIN_PASSWORD` | No | Random UUID | Optional password for the temporary macOS keychain created by `import-cert.sh`. |
| `APPLE_ID` | For Notarization | *None* | Apple ID email address used by `xcrun notarytool submit`. |
| `APPLE_PASSWORD` | For Notarization | *None* | App-specific password (`xxxx-xxxx-xxxx-xxxx`) generated at `appleid.apple.com` for `notarytool`. |
| `APPLE_TEAM_ID` | For Notarization | *None* | 10-character Apple Developer Team ID. (All three of `APPLE_ID`, `APPLE_PASSWORD`, and `APPLE_TEAM_ID` must be set to trigger notarization and stapling.) |

#### 3. Windows Packaging & Authenticode Signing Variables (`packaging/windows/`)

`packaging/windows/package.ps1` statically links the MSVC C runtime (`-C target-feature=+crt-static`), verifies PE COFF headers (`IMAGE_SUBSYSTEM_WINDOWS_GUI = 2` for `template-desktop.exe`, `IMAGE_SUBSYSTEM_WINDOWS_CUI = 3` for `template-cli.exe`), and invokes `packaging/windows/sign.ps1` on both the `.exe` binaries and the final `.msi`.

`sign.ps1` supports **two signing modes** (and skips signing with a notice if neither is configured):

| Variable | Mode | Default | Description |
| :--- | :---: | :--- | :--- |
| `TEMPLATE_REQUIRE_WINRES` | Build | `1` in `package.ps1` | When `1` or `true`, forces `build.rs` (`winresource`) to fail the build if `assets/app-icon/template-app.ico` or `rc.exe` is missing. |
| `SIGNTOOL` | Both | Auto-detected in Windows Kits (`x64`/`arm64`) | Explicit path to `signtool.exe`. |
| `WINDOWS_TIMESTAMP_URL` | Both | `http://timestamp.digicert.com` | RFC 3161 Authenticode timestamp server URL. |
| `WINDOWS_CERTIFICATE` | **Mode B (.pfx)** | *None* | Base64-encoded `.pfx` code-signing certificate or file path to a `.pfx` file. |
| `WINDOWS_CERTIFICATE_PASSWORD` | **Mode B (.pfx)** | `""` | Password for the `.pfx` certificate. |
| `AZURE_TENANT_ID` | **Mode A (Azure)** | *None* | Azure Entra ID Tenant ID for Azure Trusted Signing. |
| `AZURE_CLIENT_ID` | **Mode A (Azure)** | *None* | Azure Service Principal Application (Client) ID. |
| `AZURE_CLIENT_SECRET` | **Mode A (Azure)** | *None* | Azure Service Principal Client Secret. |
| `AZURE_SIGNING_ENDPOINT` | **Mode A (Azure)** | *None* | Azure Trusted Signing endpoint URI (e.g. `https://eus.codesigning.azure.net`). |
| `AZURE_SIGNING_ACCOUNT` | **Mode A (Azure)** | *None* | Azure Trusted Signing Account name. |
| `AZURE_CERT_PROFILE` | **Mode A (Azure)** | *None* | Azure Trusted Signing Certificate Profile name. |

#### 4. Linux Packaging & Cross-Compilation Variables (`packaging/linux/`)

| Variable | Required | Default | Description |
| :--- | :---: | :--- | :--- |
| `TEMPLATE_MAINTAINER` | No | `Example Team <team@example.com>` | Maintainer string written into `.deb` and `.rpm` metadata via `nfpm.yaml`. |
| `CROSS_ARCH` | No | `$(uname -m)` | Target architecture override (`x86_64` or `aarch64`) when cross-packaging. |
| `CROSS_TARGET` | No | `<arch>-unknown-linux-gnu` | Rust target triple override (e.g. `aarch64-unknown-linux-gnu`). |
| `CROSS_COMPILE` | No | `""` | Binutils prefix for `strip` when cross-compiling (e.g. `aarch64-linux-gnu-`). |
| `APPIMAGETOOL` | No | `appimagetool` | Path or binary name for `appimagetool`. |

#### 5. FreeBSD Packaging Variables (`packaging/freebsd/`)

| Variable | Required | Default | Description |
| :--- | :---: | :--- | :--- |
| `PREFIX` | No | `/usr/local` | Installation prefix used by the `install.sh` script bundled inside the FreeBSD tarball. |

---

### Customizing OS Distribution Settings for Your App

When creating a new application from `native-template`, update the following OS-specific configuration points:

1. **Application Identity & Reverse-DNS Bundle ID (`com.example.templateapp`):**
   - **macOS:** `packaging/macos/Info.plist.in` (`CFBundleName`, `CFBundleDisplayName`, `CFBundleExecutable`, `LSMinimumSystemVersion` default `11.0`, file associations in `CFBundleDocumentTypes`) and `APP_BUNDLE_ID` in `packaging/macos/package.sh`.
   - **Linux / FreeBSD:** Rename and edit `packaging/linux/com.example.templateapp.desktop`, `packaging/linux/com.example.templateapp.metainfo.xml.in`, and `packaging/linux/com.example.templateapp.mime.xml`, and update their references in `packaging/linux/nfpm.yaml`, `packaging/linux/package.sh`, and `packaging/freebsd/package.sh`.
2. **Windows WiX v5 Installer (`packaging/windows/template-app.wxs`):**
   - **Generate a new `UpgradeCode` GUID:** Replace `UpgradeCode="D4E5F6A7-8901-4B23-9C45-6789ABCDEF01"` with a fresh UUID v4 for your product and **never change it** across future releases so Windows Installer cleanly upgrades existing installations in place.
   - **Registry & Install Paths:** Update `Manufacturer`, `Name`, `<StandardDirectory Id="ProgramFiles6432Folder">`, and `HKLM\Software\TemplateApp\Installer` (which persists the user's opt-in Desktop Shortcut choice across upgrades).
   - **Embedded Icon & `VERSIONINFO`:** Copy `packaging/windows/app-build.rs.in` to `apps/desktop/build.rs` and add `winresource = "0.1"` under `[target.'cfg(windows)'.build-dependencies]` in `apps/desktop/Cargo.toml`.
   - **GUI Subsystem Header:** Ensure `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` is present at the top of `apps/desktop/src/main.rs` so release builds do not allocate a console window (`package.ps1` verifies `Subsystem == 2` in the PE header).
3. **Linux Shared Library Dependencies (`packaging/linux/nfpm.yaml`):**
   - `eframe` / `winit` / `wgpu` dynamically load Wayland/X11 and Vulkan/GL via `dlopen`. Keep `libc6`, `libgcc-s1`, `libxkbcommon0`, and `libvulkan1 | libegl1` in `depends`, and display server libraries (`libwayland-client0`, `libx11-6`, `libxcursor1`, `libxi6`, `libxrandr2`) in `recommends`.
   - Update `UPDATE_INFO` in `packaging/linux/package.sh` (`gh-releases-zsync|<org>|<repo>|latest|...`) so `AppImageUpdate` and Gear Lever can perform binary delta updates via `.zsync`.
4. **Web Static Hosting Headers (`packaging/web/_headers` & `.htaccess`):**
   - Serves `/*.wasm` with `Content-Type: application/wasm` and caches fingerprinted `.wasm`/`.js` assets immutably (`max-age=31536000, immutable`) while keeping `/index.html` on `Cache-Control: no-cache`.

---

## Architectural Rules

- **Zero UI below L6:** `egui`, `eframe`, `winit`, and `rfd` are strictly prohibited in layers L0 through L5.
- **Zero Networking below L6:** `reqwest`, `tokio`, and `tungstenite` are prohibited in layers L0 through L5.
- **"Never Crash" Standard:** Production code forbids `unsafe` (`unsafe_code = "forbid"`) and denies all `unwrap()`, `expect()`, `panic!()`, `todo!()`, and `unimplemented!()`.
