# Client App Beautification Workflow

This document is the working agreement for improving the OpenAaaS Client UI. Use it before making visual changes so future rounds stay consistent.

## Product Direction

OpenAaaS Client is a desktop operations app for service discovery, task submission, task tracking, and server management. The interface should feel clear, professional, and efficient rather than promotional.

Prioritize:

- Dense but readable layouts for repeated desktop use.
- Strong information hierarchy for service status, permissions, and task state.
- Polished controls with predictable hover, active, disabled, and focus states.
- Consistent spacing and component behavior across all views.

Avoid:

- Landing-page composition, oversized marketing sections, and decorative-only visuals.
- One-off colors or page-specific component styles.
- Emoji icons in product navigation or controls.
- Nested cards unless the inner surface is a genuine repeated item or form control.

## Visual Baseline

Primary brand color: Dodger Blue, `#1e90ff`.

Secondary brand color: Payne Gray, `#536878`.

Current Tailwind tokens live in `tailwind.config.js`:

- `accent`: primary interaction color.
- `secondary`: pink supporting action and attention color.
- `info`: deep violet emphasis color for titles and high-priority metadata.
- `brand.blue`: canonical Dodger Blue.
- `brand.payne`: canonical Payne Gray.
- `brand.pink`, `brand.hot`, `brand.violet`: supporting reference palette colors.
- `bg.*`: application surfaces.
- `text.*`: primary, secondary, and muted copy.
- `border.*`: standard and stronger borders.
- `success`, `warning`, `danger`: semantic state colors.

Use the Dodger Blue logo as the default brand treatment. Use the Payne Gray logo only for alternate themes or low-emphasis monochrome contexts.

The visual reference palette includes blue, pink, cyan, hot pink, and deep violet. Adopt it selectively:

- Use Dodger Blue for primary actions, active navigation, focus, and selected state.
- Use pink/hot pink for secondary emphasis, restricted access, or attention states.
- Use deep violet for page titles and important labels, but do not let it dominate full screens.
- Keep true operational success green so service/task state remains conventional and scannable.
- Keep backgrounds near-white and surfaces calm; avoid large saturated decorative blocks.

Brand source assets live in `assets/brand/open-aaas-logo/` at the repository root. Runtime client assets live in `client-app/public/`, including:

- `logo.png`: full Dodger Blue logo.
- `logo-mark-blue.png`: Dodger Blue symbol-only mark for compact UI.
- `logo-mark-payne.png`: Payne Gray symbol-only mark for alternate themes.

Local screenshot feedback lives in `test-feedback/` at the repository root and is intentionally ignored by git.

## Icon System

Use Lucide Vue icons from `@lucide/vue`.

Guidelines:

- Prefer Lucide icons for navigation, toolbar actions, and command buttons.
- Match icon stroke to the logo: line icons, rounded joins, clean silhouettes.
- Default icon size is `20px` for sidebar and compact controls.
- Use text labels or `title`/accessible names when an icon command may be ambiguous.
- Do not use emoji as app UI icons.

## Recommended Change Order

1. Establish or update the design baseline.
   Confirm colors, typography, spacing, radius, status colors, shadows, and focus states before touching individual pages.

2. Improve base components first.
   Prioritize `SideNav`, `Button`, `StatusBadge`, `ServiceCard`, `EmptyState`, `FormField`, and `Stepper`.

3. Beautify views in product-priority order.
   Start with `HomeView` because it is the service marketplace and first working surface. Continue with `ServiceDetailView`, `SubmitTaskView`, `TaskDetailView`, `TasksView`, and `SettingsView`.

4. Keep business logic stable.
   Visual changes should not alter API behavior, persistence semantics, routing, or task polling unless the user explicitly requests it.

5. Verify after every round.
   Run:

   ```bash
   npm test
   npm run build
   npm audit --omit=dev
   ```

   Then inspect the running app in a browser or Tauri window at a 1280x720 desktop viewport.

## Page-Specific Notes

### Service Marketplace

The four seed services commonly seen in local development are demo services, not production catalog data:

- `image-processing`
- `code-review`
- `doc-proofreading`
- `data-analysis`

They are useful for validating the client UI while no real Agent services are registered. When a real backend service catalog exists, these seed services should be replaced by real records from `GET /api/v1/client/services`; the client should not treat them as permanent product content.

In development, the client may append supplemental visual-only demo services when the catalog is exactly the seed demo set. These supplemental entries are only for previewing visual states and should not be persisted, submitted to, or shown as real services. They should disappear once real services are present.

The marketplace visual preview should cover these service-state combinations:

- `agent_status`: `online`, `offline`, `busy`.
- `registration_status`: `pending`, `active`, `revoked`.
- `access_type`: `public`, `restricted`.
- `has_permission`: `true`, `false`.
- load states: successful load metrics, permission-denied/failed load, and unavailable load.

Make cards scan well at a glance:

- Service name and permission should be obvious.
- Status badge should not visually dominate the card.
- Load metrics should read as compact metadata, not paragraph text.
- Restricted services should be visibly different without looking broken.

### Service Detail

Emphasize task readiness:

- Header should show service identity, status, access type, and primary action.
- Load and usage sections should have clear labels and readable markdown styling.
- The submit action should be visually stable and easy to find.

### Submit Task

Make the form feel like a guided workflow:

- Use a real stepper with labels.
- Keep service context visible but low-noise.
- Make file upload affordances clear.
- Preserve validation behavior.

### Tasks

Make state and chronology easy to scan:

- Use the four production task states in the list: queued, running, completed, and failed.
- Do not show percentage progress unless the backend provides a real progress field.
- Filter controls should look like segmented controls.
- Task rows should show title, service, status, creation time, and completion time cleanly.
- Empty states should include a useful next action when appropriate.

### Settings

Keep server management compact:

- Default and registration state should use consistent badges.
- Server cards should make URL, actions, and default state easy to identify.
- Add/register forms should use shared field and button styles once those components exist.

## Current First-Round Decisions

- Sidebar emoji icons were replaced with Lucide icons.
- `@lucide/vue` is used instead of `lucide-vue-next` because npm marks `lucide-vue-next` as deprecated.
- The default palette now uses Dodger Blue as the primary accent and Payne Gray as the secondary neutral.
- The sidebar logo uses the dedicated Dodger Blue symbol asset from `public/logo-mark-blue.png`.
- The service marketplace now uses `ServiceCard` and `StatusBadge` to absorb the reference palette in a controlled way.
