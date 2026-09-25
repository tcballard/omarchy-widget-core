# Widget authoring standard

Applies to new API 3 widgets targeting experimental Core v0.0.2. The [API contract](widget-contract.md) defines the concrete fields and commands. This document distinguishes platform enforcement from review expectations; it is not a claim of automated certification.

## Enforced platform rules

- A separate package with a stable reverse-domain ID, versioned widget.json, a QML view and optionally a QML settings editor. One definition per package, multiple independent desktop instances.
- Declare supported small/medium/large families. Core owns their frame geometry, padding, positioning, workspace assignment and cell occupancy. Footprints are 1×1, 2×1 and 2×2 on 192-logical-unit cells; inherited gaps determine the final dimensions. Usable content is smaller and typography/padding remain theme-scaled.
- Use widgetContext for acknowledged settings, identity, active state, theme/metrics and opening settings. Use settingsContext.draftSettings for the editor. Core mediates durable state and revision checks.
- Packages are bounded to 256 files / 8 MiB and settings to 8 KiB. Structural validation rejects unsafe paths and unsupported contracts; it does not review application logic.
- Package processes run behind Core's sandbox, broker, Wayland filter and resource limits. No direct network, home directory, cross-package state or raw compositor access. Same-package instances share a process and security boundary.

## UI and behaviour conventions to review

- Native QML content. Keep business/data logic separate from the view; use Rust for a helper when complexity warrants it. A widget does not need a Rust binary simply to comply.
- Content-first idle surface. For API 3, declare a settings editor and let Core supply its accessible top-right gear. API 2 compatibility content may call widgetContext.requestConfigure(). Keep configuration in the separate settings window. Core owns Save/Cancel, errors and acknowledgement. API 3 editors receive a shared Core-owned 32-unit corner gear. It reserves no header band. Leave its top-right corner clear; do not add a second gear.
- Use Core's semantic colours, typography, spacing and appearance values; avoid hard-coded dark-only styling. Core owns the surrounding frame. Widget-specific content may differ: a clock need not look like a calendar.
- Work at every declared family, including theme scaling. Declare only supported sizes. Clip/scroll deliberately; no arbitrary outer resizing or unreadable shrinking to force content to fit.
- Suspend background work when inactive, bound helper processes and requests, discard obsolete responses, and avoid constant animation or per-second updates without a product reason.
- Show honest loading, empty, unavailable, stale and error states where relevant. Never present failed or cached data as fresh. Preserve settings across updates and document any migration assumptions.
- Label controls, support keyboard interaction in settings, maintain visible focus and text contrast, and avoid using colour as the only indicator. Icons need accessible names.
- Preserve unrelated settings when changing a draft. Validate widget-specific fields. Never write state files directly from the renderer.

## Verification expectations

Before review, exercise data logic, settings drafts/cancellation, save failure and reopening, all declared sizes, light/dark themes, and hidden/inactive behaviour. Label fixtures and synthetic captures explicitly.

Before declaring desktop support, verify the real package through Core on Omarchy: install/update/rollback, actual Save acknowledgement and persistence after restart, gear/settings lifecycle, workspace changes, scaling, multiple monitors and normal resource consumption. Unit tests and offscreen Qt renders do not replace this acceptance.

## Future network widgets

Network support is proposed in [network-widgets.md](network-widgets.md), not currently available. It should add named, permissioned data resources, bounded refresh and cache/stale metadata through Core. Widget authors must not bypass the existing isolation to fetch data.

## Cell-grid layout

The implemented [cell-grid layout](grid-layout.md) uses occupied footprints and inherits global desktop gaps/borders/rounding through Core. Authors should keep layouts responsive to the actual content rectangle and avoid introducing their own exterior spacing.
