# Cell grid and desktop spacing

Status: implemented in the Core v0.0.2 development branch; real Omarchy desktop acceptance is pending.

## Desktop settings are the source of spacing

Core inherits the effective Omarchy/Hyprland look-and-feel settings: inner gaps for widget separation, outer gaps for the usable desktop inset, border size and rounding for the shared widget frame. Do not hard-code a second desktop gap setting in each widget or execute a user's Lua configuration to extract values. Read resolved values through trusted Core; renderer sandboxes keep no compositor control socket.

The inspected Quattro defaults use gaps_in=5, gaps_out=10, border_size=2 and rounding=0. Those are examples, not runtime constants. The no-gaps preset sets all four to zero, which must also produce zero widget separation, no additional outer gap, no border and square corners. Themes and personal overrides can change the effective values.

Core sums opposite inner edges: gapX = left + right, gapY = top + bottom. A scalar gaps_in=5 produces 10 logical units between frames. Outer gaps are CSS top/right/bottom/left; monitor reserved space is left/top/right/bottom and already logical. Width/height are divided by monitor scale once, with odd transforms swapping the axes. Full-screen layer geometry uses ExclusionMode.Ignore to avoid applying the reserved panel area again.

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

Desktop border size and rounding take precedence over theme and per-instance appearance values. Other widgets.json tokens remain supported. Theme scale changes content typography/padding, not the 192-logical-unit cells or inherited desktop spacing. A fixed grid is anchored at the usable top-left; leftover space at the right/bottom is deliberately unused.

## Persistence and API

Registry version 2 adds placement.cell = {column,row}; placement.monitor is the preferred output. Existing x/y fields are retained as legacy data. When desktop geometry becomes available, pixel-only placements migrate once under the registry lock. Settings and settings revisions are preserved.

`place INSTANCE_ID '{"column":1,"row":0,"monitor":"DP-1","size":"medium"}'` commits an explicit preference only if it fits and does not collide. The former pixel-based place payload is rejected with a format error. Add/duplicate find cells; a full desktop retains the enabled instance as unplaced. Hiding releases occupancy. Re-enabling retains its preference and has lower allocation priority than already enabled instances.

`list` returns desktop.grids, desktop.frame, anonymous occupancy, and per-entry effective and occupancyIndex fields. Effective geometry is transient and never written over the preference. Null effective means disabled/unplaced. Reserve viable preferences first, then fill row-major slots on the preferred output, followed by other outputs in name order. Activation order, then instance ID, resolves legacy conflicting preferences. Workspace changes validate against current effective occupants and fail atomically on a conflict.

Trusted Core issues only monitors and four fixed getoption queries. Both older custom and current css gap responses are accepted; numbered workspace id/address formats are supported. Each response is capped at 64 KiB with a 100 ms read budget, cached for 250 ms. Missing/invalid desktop data leaves every widget unplaced, preserving settings; Core does not guess spacing or allow unvalidated placement. Anonymous occupancy contains geometry/workspace only, never another package's identity or settings.

Inheritance follows resolved **global** general:gaps_in, general:gaps_out, general:border_size and decoration:rounding. Per-window/workspace rule exceptions are not applied to desktop widgets. The grid is independent of the currently tiled application layout.

Implementation references inspected at Hyprland commit `92b82c0c1e4168d93903ec42a2276843bbd84821`:
- [IPC monitor and option formats](https://github.com/hyprwm/Hyprland/blob/92b82c0c1e4168d93903ec42a2276843bbd84821/src/ipc/s1/Commands.cpp)
- [CSS gap serialization](https://github.com/hyprwm/Hyprland/blob/92b82c0c1e4168d93903ec42a2276843bbd84821/src/config/shared/complex/ComplexDataTypes.hpp)
- [Adjacent window edge gaps](https://github.com/hyprwm/Hyprland/blob/92b82c0c1e4168d93903ec42a2276843bbd84821/src/layout/target/WindowTarget.cpp)
- [Reserved area and outer gaps](https://github.com/hyprwm/Hyprland/blob/92b82c0c1e4168d93903ec42a2276843bbd84821/src/layout/space/Space.cpp)

