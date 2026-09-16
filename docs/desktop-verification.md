# Desktop integration verification

Milestone 3 builds on the shared placement engine: a disconnected preferred output
never changes the saved cell, monitor or workspace. Legacy pixel positions wait
for their own monitor before migration; fallback geometry cannot overwrite them.
Unknown Qt outputs fail closed. Geometry changes cancel a drag and reset its
preview; unchanged polling snapshots do not interrupt it.

Portable checks: 34 Rust tests pass (two socket-binding tests excluded because
this environment rejects Unix bind), strict Clippy and QML smoke pass. A 64-case
matrix covers four scales, eight transforms and docking states, checking bounds,
non-overlap, preference retention and compositor-unavailable behaviour. Existing
workspace/resize tests cover collision rejection without moving neighbours.

Live acceptance remains required: dock/undock during a drag, fractional scale,
panel reservation changes, workspace switching, theme/ricing changes and restart
on the target Hyprland desktop. These are not claimed by the portable matrix.
