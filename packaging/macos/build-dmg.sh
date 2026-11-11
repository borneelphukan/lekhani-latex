#!/usr/bin/env bash
#
# Build a universal macOS disk image for Lekhani Latex.
#
# BINARY is expected to be a universal (x86_64 + arm64) executable, normally
# produced with `lipo` from the two per-target release binaries.
#
# Environment:
#   VERSION   package version (required)
#   BINARY    path to the universal binary (default: target/release/lekhani-latex)
#   OUT_DIR   output directory (default: dist)
#
# Usage: VERSION=0.0.4 packaging/macos/build-dmg.sh

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd -- "$SCRIPT_DIR/../.." && pwd)"

VERSION="${VERSION:?VERSION is required}"
BINARY="${BINARY:-$ROOT/target/release/lekhani-latex}"
OUT_DIR="${OUT_DIR:-$ROOT/dist}"

APP_NAME="Lekhani Latex"
BUNDLE_ID="io.github.borneelphukan.lekhaniLatex"

die() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

[ -f "$BINARY" ] || die "universal binary not found at $BINARY"
BINARY="$(cd -- "$(dirname -- "$BINARY")" && pwd)/$(basename -- "$BINARY")"
command -v sips >/dev/null 2>&1 || die "sips is required (run this on macOS)"
command -v iconutil >/dev/null 2>&1 || die "iconutil is required (run this on macOS)"
command -v hdiutil >/dev/null 2>&1 || die "hdiutil is required (run this on macOS)"

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

APP_DIR="$STAGE/$APP_NAME.app"
install -d -m 0755 "$APP_DIR/Contents/MacOS" "$APP_DIR/Contents/Resources"
install -m 0755 "$BINARY" "$APP_DIR/Contents/MacOS/lekhani-latex"

# Build a multi-resolution .icns from the PNG. Crop it to a square first so the
# icons are not stretched.
SQUARE="$STAGE/icon-square.png"
ICONSET="$STAGE/icon.iconset"
sips -c 1024 1024 "$ROOT/assets/logo.png" --out "$SQUARE" >/dev/null
install -d -m 0755 "$ICONSET"
for size in 16 32 128 256 512; do
    sips -z "$size" "$size" "$SQUARE" --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
    sips -z "$((size * 2))" "$((size * 2))" "$SQUARE" \
        --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$APP_DIR/Contents/Resources/icon.icns"

cat > "$APP_DIR/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>$APP_NAME</string>
    <key>CFBundleDisplayName</key>
    <string>$APP_NAME</string>
    <key>CFBundleIdentifier</key>
    <string>$BUNDLE_ID</string>
    <key>CFBundleVersion</key>
    <string>$VERSION</string>
    <key>CFBundleShortVersionString</key>
    <string>$VERSION</string>
    <key>CFBundleExecutable</key>
    <string>lekhani-latex</string>
    <key>CFBundleIconFile</key>
    <string>icon.icns</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.productivity</string>
</dict>
</plist>
EOF

mkdir -p "$OUT_DIR"
OUTPUT="$OUT_DIR/LekhaniLatex-${VERSION}-macos-universal.dmg"
hdiutil create \
    -volname "$APP_NAME" \
    -srcfolder "$APP_DIR" \
    -ov \
    -format UDZO \
    "$OUTPUT" >/dev/null

printf 'built %s\n' "$OUTPUT"
