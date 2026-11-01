#!/usr/bin/env bash
# Build Linux release artifacts:
#   dist/template-app-<version>-linux-<arch>.tar.gz
#   dist/template-app_<version>_<deb-arch>.deb          (if nfpm is installed)
#   dist/template-app-<version>-1.<rpm-arch>.rpm        (if nfpm is installed)
#   dist/TemplateApp-<version>-<appimage-arch>.AppImage (if appimagetool is installed)
#
# Run on the target architecture, or set CROSS_ARCH=aarch64 + CROSS_TARGET=aarch64-unknown-linux-gnu.
# Set `--skip-build` to package already-built binaries in target/<rust-target>/release.
set -euo pipefail

# shellcheck source=../env.sh
source "$(cd "$(dirname "$0")/.." && pwd)/env.sh"

SKIP_BUILD=0
for arg in "$@"; do
  case "$arg" in
    --skip-build) SKIP_BUILD=1 ;;
    *) echo "usage: $0 [--skip-build]" >&2; exit 2 ;;
  esac
done

ARCH="${CROSS_ARCH:-$(uname -m)}"
case "$ARCH" in
  x86_64|amd64)
    ARCH="x86_64"; DEB_ARCH="amd64"; RPM_ARCH="x86_64"; APPIMAGE_ARCH="x86_64"
    RUST_TARGET="${CROSS_TARGET:-x86_64-unknown-linux-gnu}"
    ;;
  aarch64|arm64)
    ARCH="aarch64"; DEB_ARCH="arm64"; RPM_ARCH="aarch64"; APPIMAGE_ARCH="aarch64"
    RUST_TARGET="${CROSS_TARGET:-aarch64-unknown-linux-gnu}"
    ;;
  *) echo "unsupported Linux arch: $ARCH" >&2; exit 1 ;;
esac

echo "==> Building TemplateApp $VERSION for Linux ($ARCH / $RUST_TARGET)"

if [[ "$SKIP_BUILD" -eq 0 ]]; then
  cargo build --release --target "$RUST_TARGET" -p template-desktop -p template-cli
fi

BIN_DIR="$CARGO_TARGET_DIR/$RUST_TARGET/release"
GUI_BIN="$BIN_DIR/template-desktop"
CLI_BIN="$BIN_DIR/template-cli"
for bin in "$GUI_BIN" "$CLI_BIN"; do
  if [[ ! -x "$bin" ]]; then
    echo "error: expected binary missing: $bin" >&2
    exit 1
  fi
done

STRIP="${CROSS_COMPILE:-}strip"
if command -v "$STRIP" >/dev/null 2>&1; then
  "$STRIP" "$GUI_BIN" "$CLI_BIN" || warn "$STRIP failed; shipping unstripped binaries"
else
  warn "$STRIP not found; shipping unstripped binaries"
fi

# Verify ELF machine matches target arch so a misconfigured cross build cannot
# silently ship an x86_64 binary inside an arm64 package.
if command -v readelf >/dev/null 2>&1; then
  ELF_MACHINE="$(readelf -h "$GUI_BIN" | awk -F: '/Machine:/ {gsub(/^[ \t]+/, "", $2); print $2}')"
  case "$ARCH:$ELF_MACHINE" in
    x86_64:*X86-64*) ;;
    aarch64:*AArch64*) ;;
    *) echo "error: $GUI_BIN ELF machine '$ELF_MACHINE' does not match target arch $ARCH" >&2; exit 1 ;;
  esac
fi

STAGE="$(mktemp -d "${TMPDIR:-/tmp}/template-app-linux.XXXXXX")"
trap 'rm -rf "$STAGE"' EXIT

# --- Populate FHS tree under $STAGE/usr (shared by tarball, deb, rpm, AppDir) ---
USR="$STAGE/usr"
install -Dm755 "$GUI_BIN" "$USR/bin/template-desktop"
install -Dm755 "$CLI_BIN" "$USR/bin/template-cli"
install -Dm644 "$ROOT/packaging/linux/com.example.templateapp.desktop" \
  "$USR/share/applications/com.example.templateapp.desktop"
BUILD_DATE_ISO="$(date -u +%Y-%m-%d)"
sed -e "s/@VERSION@/$VERSION/g" -e "s/@DATE@/$BUILD_DATE_ISO/g" \
  "$ROOT/packaging/linux/com.example.templateapp.metainfo.xml.in" \
  > "$STAGE/com.example.templateapp.metainfo.xml"
install -Dm644 "$STAGE/com.example.templateapp.metainfo.xml" \
  "$USR/share/metainfo/com.example.templateapp.metainfo.xml"
install -Dm644 "$ROOT/packaging/linux/com.example.templateapp.mime.xml" \
  "$USR/share/mime/packages/com.example.templateapp.xml"

if [[ -f "$ROOT/assets/app-icon/template-app.svg" ]]; then
  install -Dm644 "$ROOT/assets/app-icon/template-app.svg" \
    "$USR/share/icons/hicolor/scalable/apps/com.example.templateapp.svg"
fi
for sz in 16 24 32 48 64 128 256 512; do
  png="$ROOT/assets/app-icon/hicolor/${sz}x${sz}/apps/com.example.templateapp.png"
  if [[ -f "$png" ]]; then
    install -Dm644 "$png" \
      "$USR/share/icons/hicolor/${sz}x${sz}/apps/com.example.templateapp.png"
  fi
done
copy_docs "$USR/share/doc/template-app"

# Validate desktop + metainfo files when validators are present in the build env
if command -v desktop-file-validate >/dev/null 2>&1; then
  desktop-file-validate "$USR/share/applications/com.example.templateapp.desktop"
fi
if command -v appstreamcli >/dev/null 2>&1; then
  appstreamcli validate --no-net "$USR/share/metainfo/com.example.templateapp.metainfo.xml" \
    || warn "appstreamcli validate reported warnings"
fi

# --- 1. Portable .tar.gz ---
TAR_NAME="template-app-${VERSION}-linux-${ARCH}"
TAR_ROOT="$STAGE/$TAR_NAME"
mkdir -p "$TAR_ROOT"
cp -R "$USR/bin" "$USR/share" "$TAR_ROOT/"
cat > "$TAR_ROOT/install.sh" <<'EOF'
#!/usr/bin/env bash
# Install TemplateApp into $PREFIX (default: ~/.local for non-root, /usr/local for root).
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
if [[ -z "${PREFIX:-}" ]]; then
  if [[ "$(id -u)" -eq 0 ]]; then PREFIX="/usr/local"; else PREFIX="$HOME/.local"; fi
fi
echo "Installing TemplateApp to $PREFIX ..."
mkdir -p "$PREFIX/bin" "$PREFIX/share"
cp -f "$HERE/bin/template-desktop" "$HERE/bin/template-cli" "$PREFIX/bin/"
cp -Rf "$HERE/share/"* "$PREFIX/share/"
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$PREFIX/share/applications" || true
command -v update-mime-database    >/dev/null 2>&1 && update-mime-database    "$PREFIX/share/mime"         || true
command -v gtk-update-icon-cache   >/dev/null 2>&1 && gtk-update-icon-cache -f -t "$PREFIX/share/icons/hicolor" || true
echo "Done. Run '$PREFIX/bin/template-desktop' or launch TemplateApp from your application menu."
EOF
chmod 755 "$TAR_ROOT/install.sh"

TAR_OUT="$DIST/${TAR_NAME}.tar.gz"
tar -C "$STAGE" -czf "$TAR_OUT" "$TAR_NAME"
sha256 "$TAR_OUT"
echo "    wrote $TAR_OUT"

# --- 2. .deb and .rpm via nfpm ---
if command -v nfpm >/dev/null 2>&1; then
  export VERSION DEB_ARCH RPM_ARCH STAGE_USR="$USR"
  export TEMPLATE_MAINTAINER="${TEMPLATE_MAINTAINER:-Example Team <team@example.com>}"
  DEB_OUT="$DIST/template-app_${VERSION}_${DEB_ARCH}.deb"
  RPM_OUT="$DIST/template-app-${VERSION}-1.${RPM_ARCH}.rpm"
  nfpm package --config "$ROOT/packaging/linux/nfpm.yaml" --packager deb --target "$DEB_OUT"
  nfpm package --config "$ROOT/packaging/linux/nfpm.yaml" --packager rpm --target "$RPM_OUT"
  sha256 "$DEB_OUT"
  sha256 "$RPM_OUT"
  echo "    wrote $DEB_OUT"
  echo "    wrote $RPM_OUT"
else
  warn "nfpm not installed; skipping .deb and .rpm (install from https://nfpm.goreleaser.com/)"
fi

# --- 3. AppImage via appimagetool ---
APPIMAGETOOL="${APPIMAGETOOL:-appimagetool}"
if command -v "$APPIMAGETOOL" >/dev/null 2>&1; then
  APPDIR="$STAGE/TemplateApp.AppDir"
  mkdir -p "$APPDIR"
  cp -R "$USR" "$APPDIR/usr"
  ln -sf usr/share/applications/com.example.templateapp.desktop "$APPDIR/com.example.templateapp.desktop"
  if [[ -f "$USR/share/icons/hicolor/256x256/apps/com.example.templateapp.png" ]]; then
    cp "$USR/share/icons/hicolor/256x256/apps/com.example.templateapp.png" "$APPDIR/com.example.templateapp.png"
    ln -sf com.example.templateapp.png "$APPDIR/.DirIcon"
  elif [[ -f "$USR/share/icons/hicolor/scalable/apps/com.example.templateapp.svg" ]]; then
    cp "$USR/share/icons/hicolor/scalable/apps/com.example.templateapp.svg" "$APPDIR/com.example.templateapp.svg"
    ln -sf com.example.templateapp.svg "$APPDIR/.DirIcon"
  fi
  cat > "$APPDIR/AppRun" <<'EOF'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
export PATH="${HERE}/usr/bin:${PATH}"
export XDG_DATA_DIRS="${HERE}/usr/share:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"
exec "${HERE}/usr/bin/template-desktop" "$@"
EOF
  chmod 755 "$APPDIR/AppRun"

  APPIMAGE_OUT="$DIST/TemplateApp-${VERSION}-${APPIMAGE_ARCH}.AppImage"
  # Embed GitHub Releases update information so AppImageUpdate / Gear Lever can
  # perform delta updates via the accompanying .zsync file.
  UPDATE_INFO="gh-releases-zsync|example|native-template|latest|TemplateApp-*-${APPIMAGE_ARCH}.AppImage.zsync"
  ARCH="$APPIMAGE_ARCH" "$APPIMAGETOOL" --no-appstream \
    -u "$UPDATE_INFO" "$APPDIR" "$APPIMAGE_OUT" \
    || ARCH="$APPIMAGE_ARCH" "$APPIMAGETOOL" --no-appstream "$APPDIR" "$APPIMAGE_OUT"
  chmod +x "$APPIMAGE_OUT"
  sha256 "$APPIMAGE_OUT"
  [[ -f "${APPIMAGE_OUT}.zsync" ]] && sha256 "${APPIMAGE_OUT}.zsync"
  echo "    wrote $APPIMAGE_OUT"
else
  warn "appimagetool not installed; skipping AppImage"
fi
