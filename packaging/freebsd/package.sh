#!/usr/bin/env bash
# Build FreeBSD release archive:
#   dist/template-app-<version>-freebsd-<arch>.tar.gz
#
# Run natively on a FreeBSD host. Installs into /usr/local by default via the
# bundled install.sh script (FreeBSD standard prefix for third-party packages).
set -euo pipefail

# shellcheck source=../env.sh
source "$(cd "$(dirname "$0")/.." && pwd)/env.sh"

ARCH="$(uname -m)"
echo "==> Building TemplateApp $VERSION for FreeBSD ($ARCH)"

cargo build --release -p template-desktop -p template-cli

BIN_DIR="$CARGO_TARGET_DIR/release"
strip "$BIN_DIR/template-desktop" "$BIN_DIR/template-cli" 2>/dev/null || true

STAGE="$(mktemp -d "${TMPDIR:-/tmp}/template-app-freebsd.XXXXXX")"
trap 'rm -rf "$STAGE"' EXIT

TAR_NAME="template-app-${VERSION}-freebsd-${ARCH}"
ROOT_DIR="$STAGE/$TAR_NAME"
mkdir -p "$ROOT_DIR/bin" \
         "$ROOT_DIR/share/applications" \
         "$ROOT_DIR/share/metainfo" \
         "$ROOT_DIR/share/mime/packages" \
         "$ROOT_DIR/share/icons/hicolor/scalable/apps"

install -m 755 "$BIN_DIR/template-desktop" "$ROOT_DIR/bin/template-desktop"
install -m 755 "$BIN_DIR/template-cli"     "$ROOT_DIR/bin/template-cli"
install -m 644 "$ROOT/packaging/linux/com.example.templateapp.desktop" \
  "$ROOT_DIR/share/applications/com.example.templateapp.desktop"
BUILD_DATE_ISO="$(date -u +%Y-%m-%d)"
sed -e "s/@VERSION@/$VERSION/g" -e "s/@DATE@/$BUILD_DATE_ISO/g" \
  "$ROOT/packaging/linux/com.example.templateapp.metainfo.xml.in" \
  > "$ROOT_DIR/share/metainfo/com.example.templateapp.metainfo.xml"
chmod 644 "$ROOT_DIR/share/metainfo/com.example.templateapp.metainfo.xml"
install -m 644 "$ROOT/packaging/linux/com.example.templateapp.mime.xml" \
  "$ROOT_DIR/share/mime/packages/com.example.templateapp.xml"
if [[ -f "$ROOT/assets/app-icon/template-app.svg" ]]; then
  install -m 644 "$ROOT/assets/app-icon/template-app.svg" \
    "$ROOT_DIR/share/icons/hicolor/scalable/apps/com.example.templateapp.svg"
fi
for sz in 16 24 32 48 64 128 256 512; do
  png="$ROOT/assets/app-icon/hicolor/${sz}x${sz}/apps/com.example.templateapp.png"
  if [[ -f "$png" ]]; then
    mkdir -p "$ROOT_DIR/share/icons/hicolor/${sz}x${sz}/apps"
    install -m 644 "$png" "$ROOT_DIR/share/icons/hicolor/${sz}x${sz}/apps/com.example.templateapp.png"
  fi
done
copy_docs "$ROOT_DIR/share/doc/template-app"

cat > "$ROOT_DIR/install.sh" <<'EOF'
#!/bin/sh
set -eu
HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
PREFIX="${PREFIX:-/usr/local}"
echo "Installing TemplateApp to $PREFIX ..."
mkdir -p "$PREFIX/bin" "$PREFIX/share"
cp -f "$HERE/bin/template-desktop" "$HERE/bin/template-cli" "$PREFIX/bin/"
cp -Rf "$HERE/share/"* "$PREFIX/share/"
echo "Done. Run '$PREFIX/bin/template-desktop'."
EOF
chmod 755 "$ROOT_DIR/install.sh"

TAR_OUT="$DIST/${TAR_NAME}.tar.gz"
tar -C "$STAGE" -czf "$TAR_OUT" "$TAR_NAME"
sha256 "$TAR_OUT"
echo "    wrote $TAR_OUT"
