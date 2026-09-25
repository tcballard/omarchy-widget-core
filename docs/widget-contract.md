# Widget contract · Core v0.0.2

Experimental API 3. See [compatibility.md](compatibility.md). Package version, manifest schema version and Core API are separate numbers. The release version reset does not reduce the API number.

## Identity and manifest

A package has a reverse-domain `id`; v0.0.2 supports one widget definition (`main`) per package. Desktop instances have independent `instanceId` values and settings. Manager Add and `create` always allocate a fresh identity; legacy `add PACKAGE` retains the package ID. `duplicate` also creates a fresh identity.

See `examples/notes/widget.json` for a complete API 3 package. Required fields: `schemaVersion:2`, `kind:"desktop-widget"`, `coreApi:3`, `id`, `name`, numeric `version`, relative QML `entryPoint`, unique `families`, `defaultFamily`, object `defaults`. Optional `settingsEntryPoint` supplies the editor body. Families are `small`, `medium`, `large`; Core owns dimensions, frame, padding and movement.

Packages must be regular directories with at most 256 files / 8 MiB, bounded nesting, no symlinks or special files. Validation is structural, not a code security review. QML can execute processes. Install only trusted code.

## Display context

The root QML item declares `required property var widgetContext`. Core passes it as an initial Loader property before component completion.

- `api`, `packageId`, `definitionId`, `instanceId`: identity and contract.
- `settings`, `settingsRevision`: last acknowledged state; do not mutate it in place.
- `family`: small, medium or large.
- `active`: whether the host considers the widget visible. Suspend background work when false.
- `theme`: Core-owned foreground, background, accent, muted and urgent colors.
- `metrics`: Core-owned font and spacing helpers.
- `appearance`: desktop-owned widget tokens; effective desktop border size and rounding take precedence.
- `requestConfigure()`: opens Core's settings surface when an editor is declared.

API 3 supports `saveSettings(object, expectedRevision)`, `saving`, `saved`, and `saveError` for display actions. Pass the settings revision observed when preparing the action; omission uses the current acknowledged revision. Legacy API 1 additionally uses `sizeName` and `requestInput(bool)`. `saveSettings` returning true means queued, not durably saved. Observe completion state. No concurrent save for the same instance is accepted. An action acknowledgement never closes an open settings draft. The draft retains its original revision; Save then reports a conflict if the action changed durable state. Cancel/reopen to load the action result. Timers should persist absolute deadlines; see `examples/countdown` for Start/Pause/Resume. The compatibility `qs.Commons` and `qs.Ui` modules belong to Core; they are not the shell's singletons and do not promise the full Quattro plugin API.

## Settings editor

The optional editor declares `required property var settingsContext`. It renders only its body and edits `draftSettings` by assigning a new object. Core owns the surrounding Save/Cancel controls, focus, busy state, errors and close-on-acknowledgement. The context also exposes `theme`, `metrics`, and `appearance`.

Cancel discards the draft. Save sends `{revision,settings}`. A stale revision fails without overwriting durable settings. Core retains the failed draft; cancel/reopen to load newer settings. The editor may expose validationError for immediate feedback. Core enforces the optional settingsSchema as well as object and byte limits at every save.

Opening the same instance's editor again retains the current draft. Finish with
Save or Cancel before opening a different instance in the same package runner.
Save is unavailable until the editor loads successfully. A save acknowledgement
belongs to the editor generation that issued it; it cannot close a later editor.
Embedded editors must let unhandled Escape events propagate to Core's Cancel
handling. Closing destroys the editor and clears its draft; reopening starts
from acknowledged settings and clears previous save errors.

## Registry commands

The Rust helper prints one JSON response and exits nonzero on failure. `list` returns API 2, registry revision, palette, appearance, installed entries and problems. Each entry includes manifest, immutable code directory, identity and placement. Each write response includes the committed placement and registry revision where applicable.

`save INSTANCE_ID JSON` requires `{revision:N,settings:{...}}`. Legacy `configure INSTANCE_ID JSON` is an unconditional replacement retained for CLI compatibility. New editors must use revisioned saves. Placement and visibility changes do not advance the settings revision.

Core serializes UI writes, coalesces queued placement requests for the same instance and deduplicates queued refreshes. A full queue rejects explicitly. External simultaneous writes are refused with a retryable busy error. A timeout is reported as uncertain because the commit may already have reached disk.

`install PATH` installs without activating. `update PATH` requires an existing package. Both validate and copy into a new complete version directory; only then does the registry pointer change. `rollback PACKAGE_ID` switches to the previous valid version. These operations accept the deliberate 0.1.x → 0.0.2 reset. The host observes registry changes within about one second. External controls use `control METHOD` through the same locked atomic registry; no session IPC directory is shared with the sandbox.

## Runtime boundary

Widget QML and its settings editor run in a per-package Bubblewrap island, outside both the main shell and the trusted Core manager. Instances of the same package share a runner. Do not depend on shell services, direct filesystem persistence or raw compositor protocols. Use `widgetContext` and the acknowledged save contract. Core supplies themes through snapshots and mediates durable state; the runner cannot access another package's settings. Settings editors use ordinary floating windows. Raw renderer network access is denied; the optional Core weather broker is permissioned separately. See runtime-and-security.md for the filtered display policy and remaining limits.

## Workspace placement extension

Placement has an optional `workspace` field: absent/null means all workspaces; integers 1–9999 select a numbered workspace on the instance's configured monitor. `workspace INSTANCE_ID all|NUMBER` changes this field without changing the widget's settings revision. Existing `place` calls preserve it and `duplicate` copies it.

Snapshots include `desktop: {available, monitors, error?}`. `monitors` maps output names to active numbered workspace IDs, with no titles or window information. The host applies workspace matching to visibility; `widgetContext.active` follows it. Workspace-hidden views remain loaded to receive lifecycle signals. An unavailable desktop snapshot leaves all widgets unplaced. The broker permits workspace assignment only for an instance owned by its package. Existing API 2 contexts and manifest fields remain unchanged.

## Cell placement extension

See [grid-layout.md](grid-layout.md) for complete geometry, migration and fallback rules. `place INSTANCE_ID JSON` now requires `{column,row,monitor,size}` with nonnegative integer cell coordinates and a supported family. Writes validate current occupancy under the registry lock. The stored `placement.cell` and `placement.monitor` are preferences; per-entry `effective` is the resolved logical rectangle or null when unplaced. Snapshot `desktop.grids` supplies cell dimensions and usable bounds, `desktop.frame` supplies border/rounding, and `occupancy` supplies anonymous rectangles for preview. An entry's `occupancyIndex` identifies its own rectangle. Renderers do not receive other packages' identities or settings through occupancy.


## Manager extension (milestone 2)

The trusted `list` response adds `catalog` (one row per installed package, with
manifest, directory, instance count and optional problem) and `retained` (saved
instances whose package was uninstalled). Existing `installed` entries keep their
shape. Catalog and retained metadata are stripped from package broker snapshots;
management operations remain unavailable to widget runners.

New commands:

- `create PACKAGE_ID FAMILY`: a fresh identity/settings object from defaults with
  a supported Small/Medium/Large family. It never reuses an existing instance.
- `remove-instance INSTANCE_ID`: delete only the named instance, including a
  retained instance. Clear any pending editor request for it. Stale revisioned
  saves, Hide, Place, Workspace and Configure cannot recreate a removed instance.
  Only explicit Add/Create allocate an initial instance; Duplicate requires an existing source.
- `uninstall PACKAGE_ID keep|delete`: unregister the package and stop its runners;
  either retain its instances hidden or delete them. Retained code directories
  are not garbage-collected. Reinstallation preserves kept settings and positions
  but never automatically shows the retained instances. A replacement package
  must support all saved instance families, or installation/update fails without
  changing the registry.

`duplicate` copies current settings and placement preferences into a new identity;
`create` uses manifest defaults. `add` still shows an existing instance or creates
the legacy package-ID instance. `remove PACKAGE_ID` retains its old destructive
meaning as an alias for `uninstall PACKAGE_ID delete`.

Optional manifest `previews` maps supported family names to relative `.png` paths,
for example `{"small":"previews/small.png"}`. Core validates package containment,
regular-file status, a maximum 512 KiB per image, PNG signature and IHDR dimensions
of 1–1024 pixels. This is structural/bounds validation, not a full PNG decoder.
The manager displays static artwork only, using a labelled footprint when absent
or undecodable. Preview QML, network URLs and executable preview generators are
not supported. The renderer is still untrusted; these assets are not a security
review or an authenticity guarantee.

## Lifecycle and public data (stack milestone 5)

Context `active` and `backgroundAllowed` are true only while the instance is
shown on its assigned workspace. `lifecycle` is visible or suspended. Signals
are resuming → becameVisible and becameHidden → suspending. Hidden workspace
views remain loaded so they receive transitions; explicit hide/removal, package
restart or process failure can destroy the view without a final signal. Never
rely on a shutdown callback for persistence. Timers should store an absolute
end time, then compute remaining time when visible again. Clock already stops
its refresh timer when inactive. All instances of one package share a process
and cgroup; cooperative lifecycle is not per-instance CPU isolation.

`refresh` declares none, minute or weather. Use active/backgroundAllowed to gate
local timers. Core enforces the weather broker's refresh budget independently
of QML. See [network-widgets.md](network-widgets.md) for the working optional
weather contract. The shared `qml/Lifecycle.qml` and `qml/DataStatus.qml` supply
signals and status presentation. Editors may expose `validationError`; Core
shows it and disables Save while nonempty. Saved data also needs schema validation.


## Authoring and recovery extensions

See [authoring-sdk.md](authoring-sdk.md) for `new PATH ID NAME`, external command
dependencies and repeatable previews. `settingsVersion`, `settingsSchema` and
`migrations` are specified in [recovery.md](recovery.md). Each settings window
registers with Core before loading. Save or Cancel it before a package update or
rollback. Explicit package restart clears a stuck editor request and discards its
unapplied draft. Only one registered settings window is opened at a time.

Frame appearance belongs to the desktop. Legacy per-instance appearance values
remain in saved settings for compatibility but do not override the shared frame.
Widget content can interpret its own settings within the frame.

API 3 configurable widgets receive a Core-owned, keyboard-accessible settings gear.
Do not add another gear in package content. API 1/2 retain their existing content controls.
