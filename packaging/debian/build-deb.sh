#!/usr/bin/env bash
#
# Build a distributable .deb for Lekhani Latex.
#
# Everything that dpkg needs is derived from Cargo metadata and from the release
# binary itself, so bumping `version` in Cargo.toml is all that is required.
#
# Environment overrides:
#   BINARY           path to the release binary (default: target/release/lekhani-latex)
#   OUT_DIR          output directory for the .deb (default: dist)
#   DEB_MAINTAINER   value for the Maintainer control field
#   DEB_ARCH         Debian architecture (default: dpkg-architecture / dpkg)
#   DEB_GLIBC_MIN    fallback minimum glibc when objdump is unavailable
#
# Usage: packaging/debian/build-deb.sh

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd -- "$SCRIPT_DIR/../.." && pwd)"
cd "$ROOT"

BINARY="${BINARY:-$ROOT/target/release/lekhani-latex}"
OUT_DIR="${OUT_DIR:-$ROOT/dist}"
DEB_ARCH="${DEB_ARCH:-$(dpkg-architecture -qDEB_HOST_ARCH 2>/dev/null || dpkg --print-architecture)}"
DEB_GLIBC_MIN="${DEB_GLIBC_MIN:-2.17}"
PKG="lekhani-latex"

log() { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
die() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

[ -f "$BINARY" ] || die "release binary not found at $BINARY (run: cargo build --release)"
BINARY="$(cd -- "$(dirname -- "$BINARY")" && pwd)/$(basename -- "$BINARY")"

if ! command -v dpkg-deb >/dev/null 2>&1; then
    die "dpkg-deb is required (apt-get install -y dpkg-dev)"
fi

# --- metadata from Cargo.toml -------------------------------------------------

if command -v cargo >/dev/null 2>&1; then
    meta="$(cargo metadata --format-version 1 --no-deps --quiet)"
else
    meta=""
fi

read_meta() {
    printf '%s' "$meta" | python3 -c '
import json, sys
key = sys.argv[1]
try:
    packages = json.load(sys.stdin)["packages"]
except Exception:
    print("")
    raise SystemExit(0)
pkg = next((p for p in packages if p["name"] == "lekhani-latex"), None)
if pkg is None:
    print("")
    raise SystemExit(0)
value = pkg.get(key, "")
if isinstance(value, list):
    # Cargo lists authors as "Name <email>" strings.
    value = ", ".join(
        v if isinstance(v, str) else v.get("name", "") or v.get("email", "")
        for v in value
    )
print(value or "")
' "$1"
}

VERSION="$(read_meta version)"
[ -n "$VERSION" ] || VERSION="$(sed -n '0,/^version[[:space:]]*=/s/^version[[:space:]]*=[[:space:]]*"\(.*\)"/\1/p' Cargo.toml)"
[ -n "$VERSION" ] || die "unable to determine package version from Cargo.toml"

SUMMARY="$(read_meta description)"
[ -n "$SUMMARY" ] || SUMMARY="A modern, easy-to-use LaTeX editor with live preview"
HOMEPAGE="$(read_meta homepage)"
[ -n "$HOMEPAGE" ] || HOMEPAGE="$(read_meta repository)"
[ -n "$HOMEPAGE" ] || HOMEPAGE="https://github.com/borneelphukan/lekhani-latex"
MAINTAINER="${DEB_MAINTAINER:-$(read_meta authors)}"
[ -n "$MAINTAINER" ] || MAINTAINER="lekhani-latex maintainers <lekhani-latex@users.noreply.github.com>"

# The long description is the multi-paragraph form of the summary; Debian wants the
# first line in Description: and the rest indented underneath.
SHORT_DESC="$SUMMARY"
LONG_DESC="$SUMMARY"

# --- runtime dependencies -----------------------------------------------------
#
# Verified against the linked/dlopen'd libraries of the release binary:
#   * linked:   libc6 (also provides libm.so.6), libgcc-s1
#   * dlopened: libX11, libX11-xcb, libXcursor, libXi, libXrender, libxcb,
#               libxkbcommon, libxkbcommon-x11, libwayland-client, libwayland-egl,
#               libEGL, libGL, libdbus-1
#
# So the X11/Wayland stack is dlopen'd at runtime rather than linked, which means
# dpkg cannot detect it with ldd. It has to be listed explicitly. Note that
# libm6 must NOT be listed: since glibc 2.34 it is provided by libc6 and is no
# longer an installable package on Ubuntu 22.04 or newer.

GLIBC_MIN="$DEB_GLIBC_MIN"
if command -v objdump >/dev/null 2>&1; then
    detected="$(objdump -T "$BINARY" 2>/dev/null | grep -oE 'GLIBC_[0-9]+\.[0-9]+' | sort -V | tail -n1 | cut -d_ -f2 || true)"
    [ -n "$detected" ] && GLIBC_MIN="$detected"
fi

DEPENDS="libc6 (>= ${GLIBC_MIN}), libgcc-s1, libdbus-1-3, libegl1, libgl1,"
DEPENDS="$DEPENDS libwayland-client0, libwayland-egl1,"
DEPENDS="$DEPENDS libx11-6, libx11-xcb1, libxcb1, libxcursor1, libxi6,"
DEPENDS="$DEPENDS libxkbcommon0, libxkbcommon-x11-0, libxrender1"

# TeX and a PDF rasterizer are only needed for compiling and previewing, so the
# editor itself remains installable without a full TeX distribution.
RECOMMENDS="texlive-latex-recommended, poppler-utils"

# --- stage the package tree ---------------------------------------------------

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
chmod 0755 "$STAGE"

install -d -m 0755 "$STAGE/DEBIAN"
install -d -m 0755 "$STAGE/usr/bin"
install -d -m 0755 "$STAGE/usr/share/applications"
install -d -m 0755 "$STAGE/usr/share/metainfo"
install -d -m 0755 "$STAGE/usr/share/doc/$PKG"

install -m 0755 "$BINARY" "$STAGE/usr/bin/$PKG"
install -m 0644 "$SCRIPT_DIR/$PKG.desktop" "$STAGE/usr/share/applications/$PKG.desktop"
install -m 0644 "$ROOT/README.md" "$STAGE/usr/share/doc/$PKG/README.md"
if [ -f "$SCRIPT_DIR/copyright.in" ]; then
    COPYRIGHT_HOLDER="${DEB_COPYRIGHT_HOLDER:-borneelphukan}"
    sed -e "s|@MAINTAINER@|$MAINTAINER|g" \
        -e "s|@HOMEPAGE@|$HOMEPAGE|g" \
        -e "s|@YEAR@|$(date -u +%Y)|g" \
        -e "s|@COPYRIGHT_HOLDER@|$COPYRIGHT_HOLDER|g" \
        "$SCRIPT_DIR/copyright.in" > "$STAGE/usr/share/doc/$PKG/copyright"
    chmod 0644 "$STAGE/usr/share/doc/$PKG/copyright"
fi
if [ -f "$ROOT/LICENSE" ]; then
    install -m 0644 "$ROOT/LICENSE" "$STAGE/usr/share/doc/$PKG/LICENSE"
fi

# assets/logo.png is the only icon; toolkits scale it as needed.
install -d -m 0755 "$STAGE/usr/share/icons/hicolor/256x256/apps"
install -m 0644 "$ROOT/assets/logo.png" \
    "$STAGE/usr/share/icons/hicolor/256x256/apps/$PKG.png"

# Keep AppStream release metadata in sync with the crate version.
sed -e "s|<release version=\"[^\"]*\" date=\"[^\"]*\"|<release version=\"$VERSION\" date=\"$(date -u +%Y-%m-%d)\"|" \
    "$SCRIPT_DIR/$PKG.metainfo.xml" > "$STAGE/usr/share/metainfo/$PKG.metainfo.xml"
chmod 0644 "$STAGE/usr/share/metainfo/$PKG.metainfo.xml"

for script in postinst prerm postrm; do
    [ -f "$SCRIPT_DIR/$script" ] || continue
    install -m 0755 "$SCRIPT_DIR/$script" "$STAGE/DEBIAN/$script"
done

INSTALLED_SIZE="$(du -sk --exclude=DEBIAN "$STAGE" | cut -f1)"

# --- control file -------------------------------------------------------------

cat > "$STAGE/DEBIAN/control" <<EOF
Package: $PKG
Version: $VERSION
Section: editors
Priority: optional
Architecture: $DEB_ARCH
Depends: $DEPENDS
Recommends: $RECOMMENDS
Installed-Size: $INSTALLED_SIZE
Maintainer: $MAINTAINER
Homepage: $HOMEPAGE
Description: $SHORT_DESC
 $LONG_DESC
 Built for Ubuntu and other Debian based distributions.
EOF
chmod 0644 "$STAGE/DEBIAN/control"

(
    cd "$STAGE"
    find usr -type f -print0 | LC_ALL=C sort -z | xargs -0 md5sum > DEBIAN/md5sums
)
chmod 0644 "$STAGE/DEBIAN/md5sums"

# --- build --------------------------------------------------------------------

mkdir -p "$OUT_DIR"
DEB_PATH="$OUT_DIR/${PKG}_${VERSION}_${DEB_ARCH}.deb"
rm -f "$DEB_PATH"

log "building $DEB_PATH"
dpkg-deb --root-owner-group -Zxz --build "$STAGE" "$DEB_PATH" >/dev/null

log "control fields"
dpkg-deb --field "$DEB_PATH" Package Version Architecture Depends Recommends | sed 's/^/    /'
log "contents"
dpkg-deb --contents "$DEB_PATH" | awk '{printf "    %s %s\n", $1, $NF}'

printf '%s\n' "$DEB_PATH"
