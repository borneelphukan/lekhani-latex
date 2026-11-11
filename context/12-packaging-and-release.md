# 12 — Packaging and Release

All packaging scripts consume a **prebuilt** binary. None of them run `cargo build` (the release workflow does). Version and architecture come from `Cargo.toml` or from environment variables set by the workflow.

## 1. Artifact matrix

| Platform | Formats | Architectures | Script / tool |
|---|---|---|---|
| Linux (Debian/Ubuntu) | `.deb` | x86_64/amd64 only in CI | `packaging/debian/build-deb.sh` |
| Linux | `.rpm` | x86_64, aarch64 | `packaging/rpm/build-rpm.sh` |
| Linux | `.tar.gz` | x86_64, aarch64 | `packaging/linux/build-tarball.sh` |
| Linux | AppImage | x86_64, aarch64 | `packaging/linux/build-appimage.sh` |
| Windows | Inno Setup installer | x64, x86, arm64 | `installer/lekhani-latex.iss` + `ISCC.exe` |
| Windows | portable `.zip` | x64, x86, arm64 | PowerShell step in `release.yml` |
| macOS | `.dmg` universal app | x86_64 + arm64 via `lipo` | `packaging/macos/build-dmg.sh` |
| apt repo | `Release`, `Packages`, pool, keyring | amd64 | `packaging/apt/build-repo.py` |
| Pages site | static React bundle | — | `website/` |

## 2. Debian package (`packaging/debian/build-deb.sh`)

Inputs (all overridable): `BINARY`, `OUT_DIR` (default `dist`), `DEB_ARCH`, `DEB_MAINTAINER`, `DEB_GLIBC_MIN`, `DEB_COPYRIGHT_HOLDER`.

Metadata comes from `cargo metadata --format-version 1 --no-deps` for the `lekhani-latex` package (version, description, homepage/repository, authors), with fallbacks parsed from `Cargo.toml` or constants. Homepage falls back to `https://github.com/borneelphukan/lekhani-latex`.

Version: from `cargo metadata`, else the first `version =` line in `Cargo.toml`.

`Depends` is built explicitly because the GUI stack is `dlopen`ed:

```text
libc6 (>= <detected or DEB_GLIBC_MIN, default 2.17>), libgcc-s1, libdbus-1-3,
libegl1, libgl1, libwayland-client0, libwayland-egl1, libx11-6, libx11-xcb1,
libxcb1, libxcursor1, libxi6, libxkbcommon0, libxkbcommon-x11-0, libxrender1
```

`Recommends`: `texlive-latex-recommended, poppler-utils`. The editor is installable without a TeX distribution.

Payload:

```text
/usr/bin/lekhani-latex
/usr/share/applications/lekhani-latex.desktop
/usr/share/metainfo/lekhani-latex.metainfo.xml   ← release version/date rewritten
/usr/share/icons/hicolor/256x256/apps/lekhani-latex.png
/usr/share/doc/lekhani-latex/README.md
/usr/share/doc/lekhani-latex/copyright           ← from copyright.in
/usr/share/doc/lekhani-latex/LICENSE             ← only if a root LICENSE file exists
DEBIAN/{control,md5sums,postinst,prerm,postrm}
```

Build details:

- `dpkg-deb --root-owner-group -Zxz --build`;
- `Installed-Size` is `du -sk` of the staged tree excluding `DEBIAN`;
- the script prints a control-fields summary and contents, and the last line is the path to the `.deb` (the CI step captures it with `tail -n1`);
- the AppStream `<release …>` line is sed-replaced with the crate version and today's UTC date **at build time**; the source file in git still says `0.0.5`;
- `assets/logo.png` is the only icon shipped (256×256 hicolor path), and CI asserts it is non-empty;
- maintainer scripts refresh desktop/icon caches and never touch `~/.config/lekhani-latex`;
- if no root `LICENSE` file exists, no `LICENSE` is shipped, even though `copyright.in` declares MIT;
- output name: `lekhani-latex_<version>_<arch>.deb`.

CI verification unpacks the `.deb` and asserts the desktop file, AppStream XML, executable, icon, `copyright`, and that each maintainer script is executable and syntactically valid.

## 3. RPM (`packaging/rpm/build-rpm.sh`)

Inputs: `VERSION` (required), `RPM_ARCH` (required), `BINARY`, `OUT_DIR`.

- Generates a spec in a temporary `rpmbuild` tree; `%global debug_package %{nil}` and `__strip /bin/true` (the binary is already stripped by `[profile.release]`).
- Installs the binary, desktop file, AppStream metadata, 256×256 icon, and README.
- Recommends `texlive-latex` and `poppler-utils`; no explicit `Requires` for GUI libraries.
- `%post`/`%postun` refresh desktop/icon caches.
- Output name: `lekhani-latex-<version>-1.<arch>.rpm`.
- The spec is generated from `packaging/debian/*` files, so Debian/RPM share desktop and AppStream metadata.

## 4. Tarball (`packaging/linux/build-tarball.sh`)

Inputs: `VERSION`, `ARCH`, `BINARY`, `OUT_DIR`.

Stages `LekhaniLatex-<version>-linux-<arch>/` containing `lekhani-latex`, `README.md`, `lekhani-latex.desktop`, `lekhani-latex.png`, and `LICENSE` **only if** the root file exists. `tar` is run with `--owner=0 --group=0 --numeric-owner`. Output: `LekhaniLatex-<version>-linux-<arch>.tar.gz`.

## 5. AppImage (`packaging/linux/build-appimage.sh`)

Inputs: `VERSION`, `ARCH` (x86_64 or aarch64), `BINARY`, `OUT_DIR`, `APPIMAGETOOL` (optional).

- Stages an `AppDir` with `usr/bin/lekhani-latex`, desktop files at both `AppDir/` and `usr/share/applications/`, and the icon at `AppDir/lekhani-latex.png` plus the hicolor path.
- Writes an `AppRun` shell wrapper that `exec`s `usr/bin/lekhani-latex "$@"` after resolving symlinks.
- If `APPIMAGETOOL` is unset, downloads `appimagetool-<arch>.AppImage` from the AppImage `continuous` GitHub release.
- Runs `ARCH=<arch> APPIMAGE_EXTRACT_AND_RUN=1 "$TOOL" --no-appstream "$APPDIR" "$OUTPUT"`.
- Output: `LekhaniLatex-<version>-<arch>.AppImage`.

`--no-appstream` means the AppImage contains no embedded AppStream XML; software-center metadata depends on the deb/RPM path.

## 6. macOS DMG (`packaging/macos/build-dmg.sh`)

Inputs: `VERSION`, `BINARY` (must be universal), `OUT_DIR`.

- Builds `Lekhani Latex.app/Contents/{MacOS,Resources}` with the executable named `lekhani-latex`.
- Creates `icon.icns` from `assets/logo.png` by cropping to a square with `sips -c 1024 1024` and generating 16/32/128/256/512 (plus `@2x`) sizes with `iconutil`.
- Writes an `Info.plist` with:
  - bundle id `io.github.borneelphukan.lekhaniLatex`,
  - `CFBundleName`/`CFBundleDisplayName` `Lekhani Latex`,
  - `CFBundleExecutable` `lekhani-latex`,
  - version fields from `VERSION`,
  - `LSMinimumSystemVersion` 11.0, `NSHighResolutionCapable`, category `public.app-category.productivity`.
- `hdiutil create -volname "Lekhani Latex" -srcfolder <app> -ov -format UDZO <out>`; output `LekhaniLatex-<version>-macos-universal.dmg`.
- No code signing, notarization, entitlements, or custom icon file (`CFBundleIconFile` is `icon.icns`, but the DMG is ad-hoc and unsigned in CI).

The universal binary comes from the workflow:

```bash
cargo build --release --locked -p lekhani-latex --target aarch64-apple-darwin
cargo build --release --locked -p lekhani-latex --target x86_64-apple-darwin
lipo -create -output target/release/lekhani-latex target/{aarch64,x86_64}-apple-darwin/release/lekhani-latex
```

`MACOSX_DEPLOYMENT_TARGET=11.0` is exported by the job.

## 7. Windows installer and portable zip

### `gui/build.rs` (binary icon)

Embeds `assets/logo.ico` via `winres` when building for Windows. CI comments explicitly note that no conversion step is needed.

### `installer/lekhani-latex.iss`

Required defines from the workflow:

| Define | Meaning |
|---|---|
| `MyAppVersion` | `AppVersion`, `AppVerName` |
| `MyVersionInfo` | Four-part version metadata, e.g. `0.0.7.0` |
| `MyArch` | `x64`, `x86`, `arm64` |
| `MySourceExe` | Absolute path to the built `.exe` |
| `MyOutputBaseFilename` | e.g. `LekhaniLatex-Setup-0.0.7-x64` |

Installer settings:

- fixed `AppId` GUID `{{9F3B4D2A-7C1E-4B6A-8D5F-3E2A1C0B9D47}`;
- publisher `Borneel Bikash Phukan`, Pages URL as publisher URL, GitHub issues/releases as support/update URLs;
- installs to `{autopf}\Lekhani Latex`, output dir `..\target`, LZMA2 solid compression, modern wizard;
- architecture gating by `MyArch` (`x64compatible`, `arm64`, or `x86compatible`);
- optional unchecked “Create desktop icon” task;
- ships the exe, `assets/logo.ico`, `README.md` in `{app}\docs`, and `LICENSE` in `{app}\docs` with `skipifsourcedoesntexist` (so its absence is tolerated);
- Start Menu entries for the app, documentation, and uninstaller;
- `[Run]` launches the app after install (`nowait postinstall skipifsilent`);
- `[UninstallDelete]` removes the whole `{app}` directory.

### Portable zip (workflow step)

PowerShell creates `target\portable-<arch>\` with `lekhani-latex.exe` and `README.md` only (no LICENSE or icon), then `Compress-Archive` to `target\LekhaniLatex-<version>-windows-<arch>.zip`.

### Naming

- Setup: `LekhaniLatex-Setup-<version>-<arch>.exe`
- Portable: `LekhaniLatex-<version>-windows-<arch>.zip`

There is no embedded manifest/version resource for the portable exe beyond what Rust/cargo generates, and no code signing.

## 8. apt repository (`packaging/apt/`)

`build-repo.py` writes:

```text
pages/apt/dists/stable/Release[.gpg], InRelease
pages/apt/dists/stable/main/binary-amd64/Packages{,.gz,.xz}
pages/apt/pool/main/l/lekhani-latex/*.deb
pages/install.sh
pages/index.html
pages/lekhani-latex-archive-keyring.gpg   (when signed)
pages/UNSIGNED                            (when unsigned)
```

Design notes:

- It uses pure Python (gzip/lzma/hashlib + `gpg`/`dpkg-deb` subprocesses), deliberately avoiding `apt-ftparchive`, which is not on stock runner images.
- `Packages` entries are built by extracting Debian control fields (`Package`, `Version`, `Architecture`, `Depends`, `Recommends`, …) with `dpkg-deb --field`; each entry adds `Filename`, `Size`, and `Sha256`.
- Uncompressed `Packages` is kept, plus `.gz`/`.xz`; `Release` carries SHA256/size for every file under `dists/stable` (except `Release`, `Release.gpg`, `InRelease`).
- Signed builds emit `InRelease`, detached `Release.gpg`, and the keyring. Unsigned builds set `Acquire-By-Hash: no` and write the `UNSIGNED` marker.
- `install.sh` is copied next to `apt/` and made executable, and an `index.html` repository page is generated (later overwritten by the website at Pages deploy time).

`install.sh` (the user-facing installer):

```bash
curl -fsSL https://borneelphukan.github.io/lekhani-latex/install.sh | sudo bash
```

- Re-executes itself with `sudo` when not root, and never reads stdin by design.
- Writes `/etc/apt/sources.list.d/lekhani-latex.sources` (deb822) pointing at `<base>/apt/`, suite `stable`, component `main`, and `Architectures: $(dpkg --print-architecture)` (falls back to `uname -m`).
- Fetches `<base>/lekhani-latex-archive-keyring.gpg`; if that request fails, it assumes an unsigned repo and emits `Trusted: yes`; otherwise it installs the keyring into `/usr/share/keyrings` and writes `Signed-By`.
- Runs `apt-get update` then `apt-get install -y lekhani-latex`.
- Options: `--print`/`-n`, `--remove` (removes repo files only, leaves the package), `--yes`/`-y`, `--help`/`-h`; `LEKHANI_LATEX_REPO_URL` overrides the base URL.

**Important mismatch:** the Pages repository is published with **amd64 only**. `install.sh` records the host architecture, so on `arm64` Debian/Ubuntu the sources entry asks apt for an `arm64` index that does not exist.

## 9. GitHub Pages layout

`publish-pages` builds:

```text
pages/                  ← website/dist copied to the root (index.html, assets/, logo.png, …)
├── apt/                ← apt repository (dists, pool)
├── install.sh          ← from build-repo.py
└── <website assets>
```

Because Pages is a **project site**, the served URL root is `https://borneelphukan.github.io/lekhani-latex/`. The website's Vite `base` is `/lekhani-latex/`, matching that path. `build-repo.py`'s `REPO_BASE` and `install.sh`'s `BASE_URL` also match it.

## 10. Release bundle and GitHub Release

The `release` job downloads every packaging artifact except `website-dist`, verifies the expected filenames (see [11-ci-cd.md](11-ci-cd.md)), writes `SHA256SUMS`, and either:

- uploads the whole set as the `release-bundle` artifact (non-tag runs), or
- creates/updates `gh release` for the tag and uploads every file with `--clobber` (tag runs).

Release titles are `Lekhani Latex <tag>` and notes are `--generate-notes`. `SHA256SUMS` is published as an asset; consumers should check it against the downloaded file.

There is no automatic changelog, no signed releases, no `deb` signing key committed, and no verification that the tag version matches `Cargo.toml`.

## 11. Version bumping checklist

`Cargo.toml` (`[workspace.package] version`) is the single source. Also review:

| Item | Auto-updated? |
|---|---|
| `.deb` / `.rpm` / tarball / AppImage / DMG / installer filenames | yes, from `VERSION`/`Cargo.toml` |
| `Info.plist` bundle version | yes |
| AppStream `<release version/date>` | yes, sed in `build-deb.sh` only; the RPM packages the source XML verbatim, so its release entry stays stale |
| apt repo metadata | yes |
| GitHub Release title | from the tag, not from `Cargo.toml` |
| `packaging/debian/lekhani-latex.metainfo.xml` in git | no (source still says `0.0.5`) |
| Website version display | no (`VITE_APP_VERSION` is passed to the build but unused) |
| In-app About version | yes, `env!("CARGO_PKG_VERSION")` |
| Windows `VersionInfoVersion` | yes, `numeric` derived from the crate version |

The metainfo source mismatch is visible to `appstreamcli` in CI only if the file remains valid XML; the stale version is a cosmetic metadata bug.

## 12. Packaging gotchas

- No root `LICENSE` file, so `.deb`/RPM/tarball/Windows installer each skip it (or install nothing in its place).
- Only one icon resolution (256×256) for Linux; the `.ico` is separate in `assets/`.
- `assets/lekhani-latex.desktop` at the root is **not** used by any packaging script; `packaging/debian/lekhani-latex.desktop` is.
- The AppImage download of `appimagetool` is from `continuous` (mutable) releases.
- DMG/AppImage/installers are unsigned and unnotarized; SmartScreen/Gatekeeper prompts are expected.
- RPM and deb have different `Recommends` names (`texlive-latex` vs `texlive-latex-recommended`); neither is a hard dependency.
- The apt repository rebuilds only from the Debian x86_64 job. A failed `linux-packages`/`windows`/`macos` job can still leave Pages updated.
- No packaging job verifies the binary actually starts (CI has no display server).
- `document.log` is tracked at the repo root and gets copied nowhere — just don't assume it is a fixture.
