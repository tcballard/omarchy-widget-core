# Widget contract · Core v0.0.2

Experimental API 2. Package version, manifest schema version and Core API are separate numbers. The release version reset does not reduce the API number.

## Identity and manifest

A package has a reverse-domain `id`; v0.0.2 supports one widget definition (`main`) per package. Desktop instances have independent `instanceId` values and settings. The first instance retains the package ID for legacy command compatibility; `duplicate` creates another identity.

See `examples/notes/widget.json` for a complete API 2 package. Required fields: `schemaVersion:2`, `kind:"desktop-widget"`, `coreApi:2`, `id`, `name`, numeric `version`, relative QML `entryPoint`, unique `families`, `defaultFamily`, object `defaults`. Optional `settingsEntryPoint` supplies the editor body. Families are `small`, `medium`, `large`; Core owns dimensions, frame, padding and movement.

Packages must be regular directories with at most 256 files / 8 MiB, bounded nesting, no symlinks or special files. Validation is structural, not a code security review. QML can execute processes. Install only trusted code.

## Display context

The root QML item declares `required property var widgetContext`. Core passes it as an initial Loader property before component completion.

- `api`, `packageId`, `definitionId`, `instanceId`: identity and contract.
- `settings`, `settingsRevision`: last acknowledged state; do not mutate it in place.
- `family`: small, medium or large.
- `active`: whether the host considers the widget visible. Suspend background work when false.
- `theme`: Core-owned foreground, background, accent, muted and urgent colors.
- `metrics`: Core-owned font and spacing helpers.
- `appearance`: theme widget tokens with optional per-instance overrides.
- `requestConfigure()`: opens Core's settings surface when an editor is declared.

Legacy API 1 additionally uses `sizeName`, `requestInput(bool)`, `saveSettings(object)`, `saving`, `saved`, and `saveError`. `saveSettings` returning true means queued, not durably saved. Observe completion state. No concurrent save for the same instance is accepted. The compatibility `qs.Commons` and `qs.Ui` modules belong to Core; they are not the shell's singletons and do not promise the full Quattro plugin API.

## Settings editor

The optional editor declares `required property var settingsContext`. It renders only its body and edits `draftSettings` by assigning a new object. Core owns the surrounding Save/Cancel controls, focus, busy state, errors and close-on-acknowledgement. The context also exposes `theme`, `metrics`, and `appearance`.

Cancel discards the draft. Save sends `{revision,settings}`. A stale revision fails without overwriting durable settings. Core retains the failed draft; cancel/reopen to load newer settings. Widget-specific field validation remains the widget author's responsibility; Core enforces object and byte limits.

## Registry commands

The Rust helper prints one JSON response and exits nonzero on failure. `list` returns API 2, registry revision, palette, appearance, installed entries and problems. Each entry includes manifest, immutable code directory, identity and placement. Each write response includes the committed placement and registry revision where applicable.

`save INSTANCE_ID JSON` requires `{revision:N,settings:{...}}`. Legacy `configure INSTANCE_ID JSON` is an unconditional replacement retained for CLI compatibility. New editors must use revisioned saves. Placement and visibility changes do not advance the settings revision.

Core serializes UI writes, coalesces queued placement requests for the same instance and deduplicates queued refreshes. A full queue rejects explicitly. External simultaneous writes are refused with a retryable busy error. A timeout is reported as uncertain because the commit may already have reached disk.

`install PATH` installs without activating. `update PATH` requires an existing package. Both validate and copy into a new complete version directory; only then does the registry pointer change. `rollback PACKAGE_ID` switches to the previous valid version. These operations accept the deliberate 0.1.x → 0.0.2 reset. The CLI wrapper asks the host to refresh after mutations.

## Runtime boundary

Widget QML runs in the shared standalone host. It must not depend on the Omarchy shell's internal objects, services or main QML engine. The shell compatibility service forwards commands only. One widget can still crash or block the shared widget process; process separation is not a per-widget security boundary.
