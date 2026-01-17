# Templates (Askama)

This folder is organized in a component-like way.

## Contexts ("props")

Templates should avoid a single giant page object. Prefer small, explicit variables:

- `chrome`: global UI chrome (version, render duration, header mode)
- `selection`: current run/env/test + lists for navigation
- `benchmarks`: list for the results table (index page)
- `bench`: optional benchmark detail (bench page)

When running as a public site with CSP enabled (`WFB_PUBLIC=1`), some partials may also receive:

- `csp_nonce`: per-request nonce string for safe inline `<script>` tags.

This keeps components honest: they only read what they need.

## Structure

- `layouts/`: base layout(s)
- `pages/`: full pages (extend layouts)
- `partials/`: HTMX partial responses and small shared fragments
- `components/`: reusable UI components (includes)

## Macros

- `partials/htmx/macros.rs.j2`: HTMX wiring helpers.
  - Prefer `htmx::wfb_results_anchor_attrs(run_id, env, test)` over manually building `href` / `hx-get` / `hx-push-url`.
- `partials/ui/macros.rs.j2`: small UI primitives for consistent markup.
  - Use these to normalize dropdown structure (details/summary/menu panel).

## Component variants

Some components support variants via a `variant` variable in scope, e.g.:

- `components/menus/env-menu.rs.j2`: `variant = "header" | "mobile"`

Pattern:

```jinja
{% set variant = "mobile" %}
{% include "components/menus/env-menu.rs.j2" %}
```
