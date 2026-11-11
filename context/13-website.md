# 13 — Website

`website/` is a React + TypeScript + Vite + Tailwind landing page deployed to GitHub Pages by `release.yml`. The Pages site also hosts the apt repository and `install.sh`.

## 1. Stack

| Concern | Choice |
|---|---|
| Framework | React 19.2 + React DOM (`react-router-dom` 7.18) |
| Language | TypeScript ~6.0, strict project via `tsconfig` |
| Build | Vite 8 + `@vitejs/plugin-react` |
| CSS | Tailwind CSS 4 via `@tailwindcss/postcss`, plus `autoprefixer` |
| Icons | `lucide-react` |
| Lint | `oxlint` (plugins `react`, `typescript`, `oxc`) |
| Build scripts | `dev: vite`, `build: tsc -b && vite build`, `lint: oxlint`, `preview: vite preview` |
| Node version in CI | 22, npm cache against `package-lock.json` |

## 2. Routing and base path

```tsx
<HashRouter>
  <Routes>
    <Route path="/" element={<Layout />}>
      <Route index element={<Home />} />
      <Route path="download" element={<Download />} />
    </Route>
  </Routes>
</HashRouter>
```

- `HashRouter` means real URLs are `…/lekhani-latex/` and `…/lekhani-latex/#/download`.
- Vite is configured with `base: '/lekhani-latex/'` so asset URLs match the GitHub **project** Pages path: `https://borneelphukan.github.io/lekhani-latex/`.
- The deployment copies `website/dist` into the Pages `pages/` directory; `publish-pages` verifies `pages/index.html`, `pages/apt/dists/stable/Release`, and `pages/install.sh` before uploading.

## 3. Pages and components

| File | Role |
|---|---|
| `src/main.tsx` | Creates the root and imports `index.css`. |
| `src/App.tsx` | Route table only. |
| `src/components/Layout.tsx` | Sticky glass header (logo, GitHub link, Download button), `<Outlet />`, footer with logo/copyright. |
| `src/pages/Home.tsx` | Hero (“Writing, through simplicity.”), feature badge, CTA buttons, HTML/CSS mock of the app, three feature cards (preview, highlighting, autocompletion). |
| `src/pages/Download.tsx` | Static table of per-platform download buttons. |
| `src/index.css` | Tailwind import, `@theme` brand palette, base typography, `.glass` utility. |

Public assets in `website/public/`: `logo.png`, `favicon.svg`, `icons.svg`.

## 4. Fonts and theme

- Google Fonts: Inter (sans) and Playfair Display (serif) loaded from `index.html`.
- Brand palette is duplicated in `src/index.css` (`@theme --color-brand-*`) and `tailwind.config.js`.
- The body background in `Layout` is `bg-[#fcfcfc]`; feature section background is white.

## 5. CI and deployment

- `website.yml` runs **only** for pull requests touching `website/**` or the workflow itself (plus manual dispatch): `npm ci` → `npm run lint` → `npm run build`.
- `release.yml` → `website` job runs for pushes, tags, dispatch, and the weekly schedule; it passes `VITE_APP_VERSION=<version>` into the build and uploads `website-dist`.
- `publish-pages` downloads `website-dist` and copies it over the apt `pages` directory, so the website controls the site root and the apt repository remains under `/apt`.

`VITE_APP_VERSION` is **not** referenced by any source file, so changing the app version does not update the website.

## 6. Known website bugs

### Download links do not work

All `Download.tsx` hrefs are hard-coded and wrong in several ways:

| Link | Problem |
|---|---|
| Windows `releases/download/windows-stable/lekhani-latexSetup.exe` | Uses a non-existent tag/asset path; the actual asset is `LekhaniLatex-Setup-<v>-<arch>.exe` on tag `v<version>`. |
| macOS `…/download/macos-stable/lekhani-latex.dmg` | Wrong tag and wrong filename (`LekhaniLatex-<v>-macos-universal.dmg`). |
| Linux deb `…/download/linux-deb/lekhani-latex_*.deb` | Wrong tag and a literal `*` in the URL. |
| Linux rpm `…/download-linux-rpm/…` | Malformed path: `/releases/download-<tag>/`, missing the `/download/<tag>/` separator. |
| Linux AppImage `…/download-linux-appimage/…` | Same malformed path and literal `*`. |
| Linux tarball `…/download-linux-tarball/…` | Same malformed path and literal `*`. |

None of these use the real release tag or the actual file names from [12-packaging-and-release.md](12-packaging-and-release.md).

### Favicon path ignores the base URL

`website/index.html` uses `<link rel="icon" type="image/png" href="/logo.png">`. On a project site this resolves to `https://<host>/logo.png`, not `https://<host>/lekhani-latex/logo.png`. Components correctly use `import.meta.env.BASE_URL` for the logo image; the HTML head does not.

### Repository identity mismatch

The website uses `https://github.com/borneelphukan/lekhani-latex`, while the desktop app uses `example/lekhani-latex` for its update/about links. Keep both consistent when fixing them (see [14-conventions-and-gotchas.md](14-conventions-and-gotchas.md)).

## 7. Dead template assets

These are committed but not imported by any source file:

- `src/App.css`
- `src/assets/hero.png`
- `src/assets/react.svg`
- `src/assets/vite.svg`

`website/README.md` is the stock Vite React template README, not project documentation. `website/dist/` and `node_modules/` are ignored by `website/.gitignore`.

## 8. Lockfile duplication

Both `package-lock.json` and `pnpm-lock.yaml` are committed. npm is authoritative (CI + release use `package-lock.json`); the pnpm lockfile is unvalidated and can drift. Do not “fix” one and assume the other is correct.

## 9. Local workflow

```bash
cd website
npm ci
npm run dev        # Vite dev server
npm run lint
npm run build
npm run preview    # serve the built site locally
```

The built site assumes the `/lekhani-latex/` base. `npm run preview` will 404 at `/` unless you request `/lekhani-latex/`.

## 10. Checklist for website changes

- Update `Download.tsx` hrefs to the real tag + filename scheme, ideally generated from release data rather than hand-written tags.
- Keep `base: '/lekhani-latex/'` unless the Pages location changes.
- Use `import.meta.env.BASE_URL` for all absolute asset paths.
- Run `npm run lint && npm run build` before pushing.
- Re-check [12-packaging-and-release.md](12-packaging-and-release.md) if asset filenames change.
