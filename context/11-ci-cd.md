# 11 — CI/CD and Workflows

Four workflows live in `.github/workflows/`: `ci.yml`, `audit.yml`, `release.yml`, and `website.yml`. They are the only automation in the repository; there is no `scripts/` directory and no local release tooling.

## 1. Workflow overview

| Workflow | Trigger | Purpose | Publishes |
|---|---|---|---|
| `ci.yml` | push to `main`, PRs, dispatch (path-filtered) | fmt + clippy + tests + packaging-asset validation | workflow artifacts only (none) |
| `audit.yml` | weekly (Sun 06:00 UTC), dependency-config changes, PRs, dispatch | `cargo audit` against `Cargo.lock` | step summary |
| `release.yml` | push to `main`, `v*` tags, PRs, dispatch, weekly Mon 05:37 UTC | build every distributable, apt repository, landing page | GitHub Release (tags), workflow artifacts (other runs), GitHub Pages |
| `website.yml` | PRs touching `website/**`, dispatch | lint + type-check + build the site | nothing (validation only) |

`release.yml` also does a Pages deployment, so a `main` push can publish site/repo content even without a tag.

## 2. `ci.yml`

### Triggers and options

- Path filters: `core/**`, `compiler/**`, `gui/**`, `**/Cargo.toml`, `Cargo.lock`, `assets/**`, `packaging/**`, `installer/**`, and the workflow itself.
- `concurrency: ci-${{ github.ref }}` with `cancel-in-progress: true`.
- `permissions: contents: read`.
- `CARGO_TERM_COLOR: always`, `RUST_BACKTRACE: 1`, `CARGO_INCREMENTAL: 0`.

### Job `check` (ubuntu-latest)

1. Install the X11/Wayland build dependencies from [10-build-run-and-config.md](10-build-run-and-config.md).
2. Pin `dtolnay/rust-toolchain@1.99.1` with `rustfmt` + `clippy`. The comment says to bump deliberately in both workflows.
3. `Swatinem/rust-cache@v2`.
4. `cargo fmt --all -- --check`.
5. `cargo clippy --workspace --all-targets -- -D warnings` (hard gate, no `--cap-lints` or `allow`s).
6. `cargo test --workspace --locked` (currently zero tests).

The release profile is deliberately not built here; every packaging job does that.

### Job `linux-assets`

Validates packaging metadata and script syntax:

- `desktop-file-validate packaging/debian/lekhani-latex.desktop`;
- `appstreamcli validate --no-net packaging/debian/lekhani-latex.metainfo.xml`;
- `bash -n` on every shell packaging script, `sh -n` on maintainer scripts, `python3 -m py_compile packaging/apt/build-repo.py`.

Adding a new packaging script means adding it here, or its syntax errors are not caught until a release.

## 3. `audit.yml`

- Runs on a weekly schedule, on PRs/pushes touching `Cargo.toml`, `Cargo.lock`, `.cargo/audit.toml`, or the workflow, and on dispatch.
- Installs `cargo-audit` via `taiki-e/install-action`.
- `cargo audit 2>&1 | tee audit.log`, then appends the log to `$GITHUB_STEP_SUMMARY`.
- Concurrency cancels PRs only, so scheduled runs are not superseded.
- By default it **fails only on vulnerabilities**. Unmaintained/unsound advisories are printed but do not fail the run; triage belongs in `.cargo/audit.toml`.

`.cargo/audit.toml` currently has `ignore = []` and all optional settings commented out. Its header says every ignore must record crate, reason, and a revisit date.

## 4. `release.yml` — job graph

```text
metadata
   ├── debian-x86_64 ─────────────┐
   ├── linux-packages (x86_64,    │
   │                 ARM64)       ├── publish-pages  (needs ci, debian-x86_64, website)
   ├── windows (x64, x86, ARM64)  │
   ├── macos-universal            │
   ├── website ───────────────────┘
   └── ci (quality gate)  ────────┴── release (needs ci + all four package jobs)
```

| Job | Runner(s) | Notes |
|---|---|---|
| `metadata` | ubuntu | Reads the first `version = "…"` in `Cargo.toml`; emits `version` and a four-part `numeric` (patch forced to `.0`) for Windows metadata. |
| `ci` | ubuntu | Runs only when the event is not `schedule`; duplicates `check` from `ci.yml` as the tag quality gate. |
| `debian-x86_64` | ubuntu-24.04 | Runs on **every** trigger, including `schedule`, so the apt repo always has a package. |
| `linux-packages` | ubuntu-24.04, ubuntu-24.04-arm | Matrix x86_64/ARM64; builds tarball + AppImage + RPM; skipped on `schedule`. |
| `windows` | windows-latest | Matrix x64/x86/ARM64 MSVC; Inno Setup installer + portable zip; skipped on `schedule`. |
| `macos-universal` | macos-15 | Two cargo builds + `lipo` + DMG; `MACOSX_DEPLOYMENT_TARGET=11.0`; skipped on `schedule`. |
| `website` | ubuntu | npm ci + `tsc -b` + `vite build`; uploads `website-dist`; runs on schedule too. |
| `publish-pages` | ubuntu | Builds apt repo from the Debian artifact, merges the website artifact, deploys Pages. |
| `release` | ubuntu | Collects non-website artifacts, verifies expected files, writes `SHA256SUMS`, publishes the GitHub Release or artifact bundle. |

`metadata`, `debian-x86_64`, and `website` have no `if:` guard, so a weekly scheduled run produces a Debian package, the site, and a Pages deployment. The other packaging jobs explicitly skip the schedule.

## 5. Toolchain pinning

- Every Rust job uses `dtolnay/rust-toolchain@1.99.1`.
- Windows/macOS jobs pass `targets: <rustup target triple>` to the action.
- `Swatinem/rust-cache@v2` keys are per-platform:
  - `linux-x86_64-release`, `linux-x86_64-release` / `linux-aarch64-release`, `windows-<target>-release`, `macos-universal-release`;
  - plain default key for `ci`/`check`.
- No `rust-toolchain.toml` exists; the pin lives only in CI.
- `cargo build --release --locked -p lekhani-latex` is used everywhere so the committed `Cargo.lock` is authoritative.

## 6. Quality gates in `ci` and `check`

Both run the same three commands:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
```

The comments in `release.yml` say to keep the two in sync. Neither job builds the release profile because the packaging jobs already do it.

## 7. `publish-pages` details

Gating (`if`):

```text
not cancelled()
and event is not pull_request
and debian-x86_64 succeeded
and website succeeded
and ci succeeded or skipped
```

So a weekly scheduled run can publish Pages (`ci` is skipped, not failed), but PRs never touch Pages.

Steps:

1. Download artifact `Debian package (x86_64)` into `dist/`.
2. Download artifact `website-dist` into `site/`.
3. Load signing key material from:
   - secret `APT_REPO_PRIVATE_KEY` (armored private key),
   - secret `APT_REPO_PASSPHRASE` (optional),
   - variable `APT_REPO_KEY_ID`.
4. Build the apt repository with `python3 packaging/apt/build-repo.py`:
   `--debs dist --out pages --suite stable --component main --origin lekhani-latex --label "Lekhani Latex"` plus GPG flags when configured.
5. Verify `pages/apt/dists/stable/Release`, `binary-amd64/Packages`, `install.sh`, `Suite/Components/Architectures` fields, and every SHA256/size in the Release.
6. `actions/configure-pages@v5` with `enablement: true`.
7. `cp -r site/. pages/` — the website takes the site root, and the apt tree (`apt/`, `install.sh`) stays where it is.
8. `upload-pages-artifact` + `deploy-pages`.
9. Print the install command to the summary.

Without a signing key the repository is published **unsigned**, a `UNSIGNED` marker is written, and `install.sh` detects the missing keyring and uses `Trusted: yes`.

## 8. `release` job details

- Needs `ci`, `debian-x86_64`, `linux-packages`, `windows`, and `macos-universal`.
- Runs on every event except `schedule`, including pull requests (artifacts are produced for PRs).
- Downloads every artifact except `website-dist` (`pattern: "!website-dist"`, `merge-multiple: true`) into `release/`.
- Verifies these **required** filename patterns (glob, not fixed strings):

```text
lekhani-latex_*_amd64.deb
lekhani-latex-*-1.x86_64.rpm
lekhani-latex-*-1.aarch64.rpm
LekhaniLatex-*-linux-x86_64.tar.gz
LekhaniLatex-*-linux-aarch64.tar.gz
LekhaniLatex-*-x86_64.AppImage
LekhaniLatex-*-aarch64.AppImage
LekhaniLatex-Setup-*-x64.exe
LekhaniLatex-Setup-*-x86.exe
LekhaniLatex-Setup-*-arm64.exe
LekhaniLatex-*-windows-x64.zip
LekhaniLatex-*-windows-x86.zip
LekhaniLatex-*-windows-arm64.zip
LekhaniLatex-*-macos-universal.dmg
```

Any missing pattern fails the job and prints the downloaded list. Extra files are still hashed.

- Builds `SHA256SUMS` over all downloaded files and validates them with `sha256sum --check --strict --quiet`.
- For non-tag events: uploads a `release-bundle` artifact (14-day retention).
- For `refs/tags/v*`: creates the release if absent (`--generate-notes`) and uploads all files with `--clobber`. The job never checks out the repo, so it exports `GH_REPO: github.repository` for `gh` to work.
- `permissions: contents: write` on the workflow; `publish-pages` adds `pages: write` and `id-token: write`.

## 9. `website.yml`

Deliberately tiny: PRs that only change `website/**` are validated without the packaging matrix. Node 22 + `npm ci` + `npm run lint` (oxlint) + `npm run build` (`tsc -b && vite build`), with `working-directory: website` and `cache-dependency-path: website/package-lock.json`.

It builds nothing for deployment; deployment happens only through `release.yml` → `publish-pages`.

## 10. Release process

There is no version-bump script and no tag-creation job.

1. Edit `version` in the root `Cargo.toml` workspace package block.
2. Run the local quality commands from [10-build-run-and-config.md](10-build-run-and-config.md).
3. Commit and push to `main` (this builds artifacts and Pages, not a GitHub Release).
4. Tag the same commit: `git tag v<version> && git push origin v<version>`.
5. `release.yml` reads the version, runs the full matrix, and attaches the bundle to a release named `Lekhani Latex v<version>`.

**Caveat:** nothing verifies that the tag matches the version read from `Cargo.toml`. Pushing `v9.9.9` on a `0.0.7` tree produces release notes for `v9.9.9` while every artifact is named `0.0.7`. See [14-conventions-and-gotchas.md](14-conventions-and-gotchas.md).

## 11. CI gotchas

- `ci` in `release.yml` and `check` in `ci.yml` must be edited together.
- Both workflows pin `1.99.1`; a new Rust version can introduce clippy lints that break unrelated tag builds.
- `cargo test --workspace --locked` is a gate that currently proves nothing.
- `linux-assets` does not validate `installer/lekhani-latex.iss`.
- The `release` job runs on PRs; CI artifacts are produced for pull requests, which increases runner time.
- Concurrency cancels in-progress runs on PRs and `main` pushes, so rapid pushes can make the last run the only one that finishes.
- Scheduled runs do **not** produce releases or most packages; they only refresh the Debian package, site, and Pages.
- The apt repository is amd64-only, while the app ships ARM64 RPM/tarball/AppImage.
- No workflow verifies the in-app update URLs or the website download links.
