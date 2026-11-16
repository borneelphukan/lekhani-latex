# syntax=docker/dockerfile:1

# Lekhani Latex — the desktop LaTeX editor in a container.
#
# The app is an eframe/egui GUI, so the image ships a headless X server
# (Xvfb) with x11vnc + noVNC: open the app from a browser. It also ships a
# TeX toolchain (pdflatex + latexmk) and poppler so Compile and the embedded
# PDF preview work out of the box.
#
#   docker build -t lekhani-latex .
#   docker run --rm -p 6080:6080 -v "$PWD:/workspace" lekhani-latex
#   # then open http://localhost:6080/vnc.html
#
# See "Run with Docker" in README.md for mounting documents and X11
# passthrough.

########################################
# Stage 1 — build the binary
########################################
FROM rust:1.99.1-bookworm AS build

# Same X11/Wayland development packages CI installs (see
# .github/workflows/ci.yml and context/10-build-run-and-config.md).
RUN apt-get update && apt-get install -y --no-install-recommends \
      pkg-config \
      libx11-dev libxcb1-dev libxcb-composite0-dev libxcb-cursor-dev \
      libxcb-image0-dev libxcb-icccm4-dev libxcb-keysyms1-dev \
      libxcb-randr0-dev libxcb-render0-dev libxcb-render-util0-dev \
      libxcb-shape0-dev libxcb-xfixes0-dev libxcb-xinerama0-dev \
      libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Compile third-party dependencies against stub workspace crates first, so
# editing source only rebuilds core/compiler/gui (a few seconds) instead of
# the full eframe/wgpu tree (many minutes).
COPY Cargo.toml Cargo.lock ./
COPY core/Cargo.toml core/
COPY compiler/Cargo.toml compiler/
COPY gui/Cargo.toml gui/build.rs gui/
RUN mkdir -p core/src compiler/src gui/src \
    && echo '' > core/src/lib.rs \
    && echo '' > compiler/src/lib.rs \
    && echo 'fn main() {}' > gui/src/main.rs \
    && cargo build --release --locked -p lekhani-latex

# Real sources. assets/logo.png is include_bytes!'d by gui/src/main.rs, so it
# must be present for the final build.
COPY assets/ assets/
COPY core/src core/src
COPY compiler/src compiler/src
COPY gui/src gui/src
RUN touch core/src/lib.rs compiler/src/lib.rs gui/src/main.rs \
    && cargo build --release --locked -p lekhani-latex

########################################
# Stage 2 — runtime with the display stack and TeX
########################################
FROM debian:bookworm-slim AS runtime

ENV HOME=/home/lekhani

# Runtime libraries mirror the explicit list in packaging/debian/build-deb.sh:
# the X11/Wayland/GL stack is dlopen'ed at runtime by egui/winit, so package
# managers cannot discover it from the binary. mesa-vulkan-drivers provides a
# software Vulkan driver (lavapipe) because eframe is built with the `wgpu`
# renderer and has no GL fallback on Linux.
RUN apt-get update && apt-get install -y --no-install-recommends \
      ca-certificates \
      libdbus-1-3 libegl1 libgl1 libwayland-client0 libwayland-egl1 \
      libx11-6 libx11-xcb1 libxcb1 libxcursor1 libxi6 libxkbcommon0 \
      libxkbcommon-x11-0 libxrender1 \
      libvulkan1 mesa-vulkan-drivers \
      texlive-latex-recommended texlive-fonts-recommended latexmk \
      poppler-utils \
      xvfb x11vnc novnc websockify openbox \
      dbus-x11 xdg-desktop-portal xdg-desktop-portal-gtk \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 1000 --user-group lekhani \
    && mkdir -p /workspace \
    && chown lekhani:lekhani /workspace

COPY --from=build /app/target/release/lekhani-latex /usr/local/bin/lekhani-latex
COPY docker/entrypoint.sh /usr/local/bin/lekhani-entrypoint.sh
RUN chmod +x /usr/local/bin/lekhani-entrypoint.sh

USER lekhani
WORKDIR /workspace

EXPOSE 6080
ENTRYPOINT ["/usr/local/bin/lekhani-entrypoint.sh"]
CMD ["lekhani-latex"]
