#!/usr/bin/env bash
#
# Build a binary RPM for a Linux build of Lekhani Latex.
#
# The binary is packaged as-is, so this works for native and cross-compiled
# targets as long as rpmbuild is available on the host.
#
# Environment:
#   VERSION    package version (required)
#   RPM_ARCH   x86_64 or aarch64 (required)
#   BINARY     path to the release binary (default: target/release/lekhani-latex)
#   OUT_DIR    output directory (default: dist)
#
# Usage: VERSION=0.0.4 RPM_ARCH=x86_64 packaging/rpm/build-rpm.sh

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd -- "$SCRIPT_DIR/../.." && pwd)"

VERSION="${VERSION:?VERSION is required}"
RPM_ARCH="${RPM_ARCH:?RPM_ARCH is required}"
BINARY="${BINARY:-$ROOT/target/release/lekhani-latex}"
OUT_DIR="${OUT_DIR:-$ROOT/dist}"

die() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

command -v rpmbuild >/dev/null 2>&1 || die "rpmbuild is required (install the 'rpm' package)"
[ -f "$BINARY" ] || die "release binary not found at $BINARY"
BINARY="$(cd -- "$(dirname -- "$BINARY")" && pwd)/$(basename -- "$BINARY")"

TOPDIR="$(mktemp -d)"
trap 'rm -rf "$TOPDIR"' EXIT
mkdir -p "$TOPDIR/BUILD" "$TOPDIR/BUILDROOT" "$TOPDIR/RPMS" \
    "$TOPDIR/SOURCES" "$TOPDIR/SPECS" "$TOPDIR/SRPMS"

SPEC="$TOPDIR/SPECS/lekhani-latex.spec"
cat > "$SPEC" <<EOF
%global debug_package %{nil}
%global __strip /bin/true

Name:           lekhani-latex
Version:        $VERSION
Release:        1
Summary:        A modern, easy-to-use LaTeX editor with live preview
License:        MIT
URL:            https://github.com/borneelphukan/lekhani-latex

Recommends:     texlive-latex
Recommends:     poppler-utils

%description
Lekhani Latex is a cross-platform desktop LaTeX editor with a live PDF
preview, syntax highlighting and command autocompletion. It supports multiple
open documents, undo and redo, automatic compilation on save, and zoomable
page-by-page preview navigation. Compilation uses the local TeX distribution.

%prep

%build

%install
set -e
install -Dm0755 "$BINARY" %{buildroot}%{_bindir}/lekhani-latex
install -Dm0644 "$ROOT/packaging/debian/lekhani-latex.desktop" \
    %{buildroot}%{_datadir}/applications/lekhani-latex.desktop
install -Dm0644 "$ROOT/packaging/debian/lekhani-latex.metainfo.xml" \
    %{buildroot}%{_datadir}/metainfo/lekhani-latex.metainfo.xml
install -Dm0644 "$ROOT/assets/logo.png" \
    %{buildroot}%{_datadir}/icons/hicolor/256x256/apps/lekhani-latex.png
install -Dm0644 "$ROOT/README.md" \
    %{buildroot}%{_datadir}/doc/lekhani-latex/README.md

%post
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database -q %{_datadir}/applications || :
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -q -f -t %{_datadir}/icons/hicolor || :
fi

%postun
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database -q %{_datadir}/applications || :
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -q -f -t %{_datadir}/icons/hicolor || :
fi

%files
%{_bindir}/lekhani-latex
%{_datadir}/applications/lekhani-latex.desktop
%{_datadir}/metainfo/lekhani-latex.metainfo.xml
%{_datadir}/icons/hicolor/256x256/apps/lekhani-latex.png
%{_datadir}/doc/lekhani-latex/README.md
EOF

rpmbuild -bb --target "$RPM_ARCH" --define "_topdir $TOPDIR" "$SPEC"

mkdir -p "$OUT_DIR"
find "$TOPDIR/RPMS" -type f -name '*.rpm' -exec cp -f {} "$OUT_DIR/" \;

printf 'built %s\n' "$OUT_DIR/lekhani-latex-${VERSION}-1.${RPM_ARCH}.rpm"
