# UI styling foundation

Looski uses plain CSS and Svelte scoped styles, without a utility framework.
The authenticated application supports light, dark, and system appearance plus
compact and comfortable density. Public marketing and authentication retain
the separate Looski brand palette.

## Ownership

`apps/web/src/app.css` imports these files from `lib/styles`:

| File           | Responsibility                                                                    |
| -------------- | --------------------------------------------------------------------------------- |
| `tokens.css`   | Default semantic colors, type, spacing, radii, motion, and component measurements |
| `base.css`     | Document defaults, typography, links, selection, and reduced motion               |
| `controls.css` | Native buttons, inputs, labels, focus, disabled states, and alerts                |
| `brand.css`    | Brand marks, eyebrows, and the skip link                                          |
| `patterns.css` | Panels, tables, and compatibility rules for existing workspace pages              |

`application.css` defines the authenticated theme boundary (`.app-frame`),
light/dark palettes, density presets, and shared list/detail patterns.
`brand-site.css` owns public brand surfaces.

`WorkspaceShell.svelte` owns the sidebar, topbar, and outer page layout. It no
longer supplies descendant styles for forms, tables, or panels. Component-specific
layout belongs in that component's scoped `<style>` block.

Styles remain unlayered for now, preserving the existing cascade between global
CSS and Svelte's scoped selectors. Do not introduce an isolated `@layer` for new
controls: unlayered legacy rules would take precedence. A future layer migration
must include the affected global and component styles together.

## Tokens and customization

Use semantic tokens for interface colors: `--surface-panel`, `--surface-input`,
`--surface-table-heading`, `--text-primary`, `--text-secondary`, `--border-default`,
`--accent`, and the `--sidebar-*` and `--error-*` families. Avoid adding literal
interface colors to pages. Decorative illustration palettes on the landing and
authentication pages are intentionally separate from the application theme.

Typography tokens use rem units; `--font-size-14` means 14px at the default 16px
root size. Spacing tokens follow a four-pixel scale: `--space-1` is 0.25rem,
`--space-2-5` is 0.625rem. Geometry such as grid tracks, breakpoints, chart sizes,
and one-off illustration positioning can remain local literals.

Prefer component measurements when changing density: `--control-height`,
`--control-padding-block`, `--control-padding-inline`, `--button-padding-inline`,
`--panel-padding`, `--panel-padding-mobile`, `--panel-gap`, and
`--table-padding-block` / `--table-padding-inline`.

Application overrides belong on `.app-frame` after the shipped presets. For example:

```css
.app-frame[data-theme='light'] {
  --accent: #2459a6;
  --accent-hover: #17427e;
  --accent-subtle: #edf3fc;
  --text-on-accent: #fff;
  --focus-ring: #457ac3;
  --focus-shadow: 0 0 0 3px #2459a620;
}
```

Change foreground, background, border, hover, active, and focus tokens together.
Derived properties resolve where they are declared: a subtree theme must redeclare
its dependent surface tokens. Check contrast in both themes and preserve semantic
status colors.

Personal preferences are validated from the `looski_theme` and `looski_density`
cookies on the server, rendered as attributes before first paint, and updated by
the shared appearance context. They last one year in the current browser. System
mode follows `prefers-color-scheme`, including changes while the page is open.
These are browser preferences, not workspace branding or cross-device settings.
No arbitrary CSS or user-supplied markup is stored.

## Shared components

Import primitives directly from `$lib/components/ui`:

- `Button.svelte`: primary/secondary variants, native button attributes and event
  handlers, and snippet content. Defaults to `type="button"`; explicitly use
  `type="submit"` for form actions. Use an anchor for navigation.
- `Panel.svelte`: a semantic section accepting native attributes and content.
  Provide an accessible name with `aria-label` or `aria-labelledby` when it is a
  named region. Keep headings in the caller so their level matches the page.
- `TableFrame.svelte`: a scroll container with shared descendant table styling.
  Keep native table markup, headings, captions, and data rendering in the caller.
- `SectionNav.svelte`: ordinary navigation links with an accessible label and
  `aria-current`. Callers supply href, label, and active state. These are page
  links, not ARIA tabs that require arrow-key navigation.

- `RecordDrawer.svelte`: a native modal dialog for create/edit forms. The browser
  provides focus containment, Escape dismissal, and focus return. Pass the action's
  draft/error state to reopen it after a failed submission; preserve idempotency
  and optimistic concurrency fields. Workflow selectors that reload the page must
  reopen their drawer from the selection query.
- `RecordName.svelte`: a compact, deterministic initial avatar and record link.
- `AppearanceControls.svelte`: the same theme/density controls in account menus
  and Settings. It reads the appearance context rather than keeping a second state.

`AccountMenu.svelte` is shared by the workspace chooser and application shell.
It shows the email returned by authenticated Auth `/user`; it never decodes an
unverified token or exposes credentials. The parent layout and child page share
workspace validation through request-scoped `event.locals`, preventing two
concurrent refreshes of one rotating token.

ProjectNav and FormNav share SectionNav and retain their route matching.
Native form controls share `controls.css`; add a field component when a repeated
label/help/error pattern needs one, rather than wrapping every input preemptively.

## Gradual migration

Existing `button`, `.button`, and `.workspace-app` descendant selectors remain
compatibility bridges. New panels and tables should use the primitives. When
editing an older page, migrate the relevant markup without changing its form
actions, input names, hidden fields, or accessibility semantics. Remove a legacy
selector only after all its consumers have migrated.

Do not add new broad rules such as `.workspace-app section button`. Prefer an
explicit component variant or local class. Keep selectors shallow and use tokens
instead of specificity overrides. Reserve `!important` for deliberate global
accessibility overrides such as reduced motion.

For foundation changes, run the web check and production build, then compare
desktop and mobile renders. Include native and component controls, table
overflow, keyboard focus, hover/disabled states, reduced motion, print styling,
and a temporary root-token override. Never ship a test route with application
fixtures or an authentication bypass.

## Public Looski brand

The landing page, authentication shell, and workspace chooser use the
`.brand-surface` theme in `brand-site.css`. It shares the spectral mark, Space
Grotesk, Instrument Serif italic, JetBrains Mono, dark stages, and warm paper
backgrounds with the `looski-v2` website. Keep these styles scoped to the theme;
business application pages continue to use the default tokens.

`BrandLogo` and `BrandFooter` are the shared identity components. The SVG and
Latin font subsets live in `apps/web/static`; font licenses are included alongside
the files. No font requests go to third-party services. Decorative colors stay
local to the public brand and its illustrative product preview.

Use `$lib/brand.ts` for public Looski, installation, source, contribution, and
contact destinations. These links are absolute so self-hosted installations
retain working project attribution and discovery links. Workspace, login,
recovery, and invitation links stay relative to the current installation.

The public landing page is independent of backend health. Keep feature copy
aligned with shipped capabilities, identify example workspace data as
illustrative, and distinguish software license fees from hosting or optional
service costs. Route leads to the existing Looski contact page rather than
collecting prospect information inside third-party installations.

## Application interaction rules

- Use short object names as page headings: Contacts, Organizations, Tasks. Keep
  warnings, permission explanations, and actual record metadata; omit taglines.
- The shell owns global navigation. Page breadcrumbs only identify a useful
  parent, and contextual navigation stays with its record or feature.
- Index pages put views and filters above a dense table, with one primary creation
  action. Creation/edit forms belong in drawers; secondary configuration can use
  explicit disclosures. Do not duplicate the page heading inside a card.
- Counts must describe the data actually loaded. `50+ shown` is a page count,
  not an invented workspace total. Search and pagination remain server backed.
- Contact identifiers truncate in the list and remain fully available in their
  title and record page. Tables scroll inside their container on smaller screens.
- Do not add decorative bulk checkboxes, fake sorting, or a record search that
  only searches navigation. The keyboard shortcut is explicitly “Go to page”.
- Keep authorization, retry keys, expected versions, and error recovery intact.
  A UI reorganization does not expand the underlying API permissions.

The index patterns draw on the operational structure of
[HubSpot record views](https://knowledge.hubspot.com/records/view-and-filter-records)
and [Salesforce data tables](https://developer.salesforce.com/docs/platform/lightning-component-reference/guide/lightning-datatable.html).
They use Looski's own components, assets, and styles.
