# UI & Templates (wfb-server)

Applies when working on frontend/UI/template/assets under `wfb-server/templates/` and `wfb-server/assets/` (and usually some handler glue under `wfb-server/src/`).

## What to inspect

- If the user mentions “frontend”, “UI”, “dashboard”, “templates”, “CSS”, “assets”, or “HTMX”, inspect both:
  - `wfb-server/templates/` + `wfb-server/assets/` (Askama templates, macros, static assets, Tailwind)
  - the corresponding web/API handlers in `wfb-server/src/` (routes/endpoints, response shapes, asset paths)
- Keep template contracts and API payloads in sync.

## UX & consistency

- Consider both mobile and desktop variants (Tailwind breakpoints/classes, conditional fragments).
- Follow existing template/component patterns; keep HTML structure and class conventions consistent.
- External links are marked with an indicator via CSS (`a[target="_blank"]`, excluding `.wfb-btn`): don’t add a dedicated SVG unless requested.

## Repository/doc links

- Do **not** hardcode repository host URLs (e.g. `github.com`) in templates or handlers.
- Use `wfb-server/src/handlers/web/types.rs` (`REPOSITORY_URL`) / `chrome.repository_url` as the single source of truth.
- In Askama templates, build repo/doc links via the `url_join` filter.

## Tailwind/Twinland notes (important)

- CSS/JS assets are built via the Rust build pipeline (`wfb-server/build.rs`): Tailwind + esbuild + fingerprinting.
  - Do NOT manually run `npx tailwindcss ...` as normal workflow.
  - Validate UI changes by running the usual Rust commands (e.g. `cargo check -p wfb-server`), which invokes the asset builder.

- `@apply` gotcha: opacity modifiers on CSS-var colors.
  - `bg-background` etc are CSS variables (`var(--background)`). Tailwind may not generate variants like `bg-background/70` for `@apply`.
  - If you need translucency + blur, prefer utilities Tailwind can generate reliably (e.g. `bg-white/70 dark:bg-black/40` + `backdrop-blur-*`).

## Header/navigation UX

- The header brand (logo + “WFB”) should always navigate to `/`.
- Don’t nest links: keep mini-nav links separate from the brand link.
- Prefer HTMX-friendly navigation (avoid forcing full reloads with `hx-boost="false"` unless there is a clear reason).
