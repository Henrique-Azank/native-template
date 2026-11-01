#!/usr/bin/env bash
# Build the self-contained WebAssembly bundle:
#   dist/template-web-<version>.zip
#
# Unzipping into any static file host (GitHub Pages, Cloudflare Pages, S3,
# nginx, `python3 -m http.server`) serves TemplateApp in the browser with no
# backend required.
#
# Prerequisites:
#   rustup target add wasm32-unknown-unknown
#   cargo install --locked trunk wasm-bindgen-cli
set -euo pipefail

# shellcheck source=../env.sh
source "$(cd "$(dirname "$0")/.." && pwd)/env.sh"

echo "==> Building TemplateApp $VERSION for Web (wasm32-unknown-unknown)"

rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true

if ! command -v trunk >/dev/null 2>&1; then
  echo "error: trunk is not installed (run: cargo install --locked trunk)" >&2
  exit 1
fi

STAGE="$(mktemp -d "${TMPDIR:-/tmp}/template-app-web.XXXXXX")"
trap 'rm -rf "$STAGE"' EXIT

BUNDLE_DIR="$STAGE/template-web-${VERSION}"
mkdir -p "$BUNDLE_DIR"

(
  cd "$ROOT/apps/web"
  # --public-url ./ makes asset paths relative so the bundle works under any
  # URL subpath (e.g. https://example.github.io/native-template/) as well as at /.
  trunk build --release --public-url ./ --dist "$BUNDLE_DIR"
)

# Verify the generated bundle is actually self-contained and sub-path safe:
# 1. at least one .wasm and one .js artifact exist,
# 2. index.html contains no root-anchored ("/...") asset URLs that would break
#    when hosted under a subpath like /native-template/.
shopt -s nullglob
wasm_files=("$BUNDLE_DIR"/*.wasm)
js_files=("$BUNDLE_DIR"/*.js)
shopt -u nullglob
if [[ ${#wasm_files[@]} -eq 0 || ${#js_files[@]} -eq 0 ]]; then
  echo "error: trunk output in $BUNDLE_DIR is missing .wasm or .js files" >&2
  exit 1
fi
if grep -Eq '(href|src)="/[^/]' "$BUNDLE_DIR/index.html"; then
  echo "error: $BUNDLE_DIR/index.html contains root-relative asset URLs; expected './' relative paths" >&2
  exit 1
fi

# Hosting hints: Cloudflare Pages / Netlify `_headers` and Apache `.htaccess`
# so `.wasm` is served with `application/wasm` and long-term caching works out
# of the box on common static hosts.
cp "$ROOT/packaging/web/_headers"  "$BUNDLE_DIR/_headers"
cp "$ROOT/packaging/web/.htaccess" "$BUNDLE_DIR/.htaccess"
copy_docs "$BUNDLE_DIR"

ZIP_OUT="$DIST/template-web-${VERSION}.zip"
rm -f "$ZIP_OUT"
(
  cd "$STAGE"
  zip -r -q "$ZIP_OUT" "template-web-${VERSION}"
)
sha256 "$ZIP_OUT"
echo "    wrote $ZIP_OUT"
