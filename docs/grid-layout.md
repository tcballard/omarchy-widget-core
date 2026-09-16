# Cell grid and desktop spacing

Status: agreed design, not implemented. Current Core still uses coordinate snapping and fixed dimensions; it does not yet enforce occupancy or inherit Hyprland gaps. This document is the specification for replacing that placement model.

## Desktop settings are the source of spacing

Core should inherit the effective Omarchy/Hyprland look-and-feel settings: inner gaps for widget separation, outer gaps for the usable desktop inset, border size and rounding for the shared widget frame. Do not hard-code a second desktop gap setting in each widget or execute a user's Lua configuration to extract values. Read resolved values through trusted Core; renderer sandboxes keep no compositor control socket.

The inspected Quattro defaults use gaps_in=5, gaps_out=10, border_size=2 and rounding=0. Those are examples, not runtime constants. The no-gaps preset sets all four to zero, which must also produce zero widget separation, no additional outer gap, no border and square corners. Themes and personal overrides can change the effective values.

Normalize the compositor's per-edge gap semantics into the actual separation between two widget frames; do not assume a raw gaps_in number is already the total visible gap. Confirm the mapping against tiled window edges on the target Hyprland build. Support asymmetric outer margins and apply monitor scaling exactly once.

Sources inspected:
- [Omarchy look-and-feel defaults](https://github.com/omacom/omarchy/blob/quattro/default/hypr/looknfeel.lua)
- [Omarchy no-gaps preset](https://github.com/omacom/omarchy/blob/quattro/default/hypr/toggles/window-no-gaps.lua)

## Footprints and geometry

- Small: 1×1 cells. Medium: 2×1. Large: 2×2.
- Each monitor gets a grid within its usable logical desktop rectangle, excluding bar/panel reserved space, then applying outer gaps. Do not subtract reserved areas twice.
- Keep the base cell size consistent in logical units; available columns/rows depend on the monitor. A 192-unit base cell is the starting point matching current Small, subject to desktop acceptance.
- A horizontal span n has width n×cellWidth + (n−1)×resolvedGapX; height uses the equivalent vertical formula. At 192 cells and a 16 gap, Medium is 400×192; the inherited gap may produce a different width.
- Widget content lays out inside Core's frame padding. Content padding is independent of the space between neighbouring widgets. No extra author-supplied outer margins or second card backgrounds.
- Changing ricing settings recomputes pixel geometry while preserving preferred cell coordinates and footprint. Widget authors must adapt to the resulting content rectangle rather than assume 400 is permanent.

## Occupancy and placement

Core owns occupancy per monitor and workspace. Enabled widgets cannot overlap. An all-workspace widget conflicts with every workspace-specific occupant in those cells; widgets assigned to different numbered workspaces may reuse them. Hidden/disabled instances retain their preferred position but do not occupy cells; re-enabling must find a valid slot.

Arrange mode shows cell targets and a footprint preview. Dropping onto occupied cells must not silently overlap or move another widget: show the invalid target and keep the last valid placement. Resizing must also validate the complete new footprint. The same checks apply to CLI and broker placement operations.

Persist preferred monitor/cell coordinates separately from a temporary effective fallback. On monitor removal, scale/resolution change or gap changes, find a deterministic valid slot without overwriting the preferred placement. Restore the preference when it becomes available again. If no slot can fit, retain the instance/settings and show an unplaced state in the manager; never overlap or shrink text to force a fit.

## Review and acceptance

Verify zero, nonzero and asymmetric gaps; bar edges; light/dark themes and rounding; 100%/fractional scaling; full grids; drag/resize collisions; workspace-specific versus all-workspace reservations; monitor loss/return; and changing ricing while widgets are present. Measure widget-to-widget and screen-edge gaps alongside ordinary windows on the XPS. Include migration of existing pixel positions without losing settings and atomic rejection of conflicting writes.

Current widgets.json appearance tokens are a Core extension. Before implementing inheritance, define and document precedence for explicit widget appearance overrides; default frames must follow the desktop rather than silently retaining today's soft rounding fallback.
