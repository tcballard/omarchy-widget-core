# Runtime and security

## Widget islands (v0.0.2 development)

Trusted Core owns installation, the registry, durable settings, layout, themes and the manager. The main Omarchy shell loads only the compatibility bridge. A Rust supervisor starts a trusted manager and a separate Quickshell runner for each active package. Instances belonging to the same package share a runner and a trust boundary. Widget views and settings editors are never loaded by the manager.

A runner is started for an enabled instance or an outstanding settings request. Replacing its package generation stops the old runner; the new runner loads the new immutable code. A runner or filter failure stops that island and allows up to three launch/failure attempts per Core session, with a five-second delay. Restart Core to reset this budget. Failures appear in `omarchy-widget logs`.

Each runner has mandatory Bubblewrap isolation: separate user, PID, IPC, network and UTS namespaces; dropped capabilities; a cleared environment; private temporary storage, `/proc` and `/dev`; and read-only system/Core runtime files. It sees exactly one package at `/widget`. It has **no mount of the registry, another package, home directory, theme directory, host runtime directory, D-Bus, SSH agent, X11 socket, Hyprland command socket or GPU devices**. Software rendering is required. There is no unsandboxed fallback.

## State broker

The supervisor creates a different private Unix listener for each package generation. Only that listener is mounted into that package's sandbox. Authority comes from the supervisor's listener association, never a package ID or path supplied in a message. Standard-library descriptors are close-on-exec; other listeners are not inherited by runner processes.

The broker accepts bounded JSON argument arrays (16 KiB), with read/write deadlines and a fixed per-loop connection budget. Its client caps responses at 2 MiB. It allows a filtered snapshot, revision-checked settings saves, placement changes, hiding owned instances and acknowledgement of owned settings requests. It also permits entering/leaving the shared arrangement UI. It rejects installation, removal, arbitrary registry operations, legacy unconditional configuration and cross-package instance IDs. An old code generation loses authority when its registry pointer changes.

Core is the only writer of durable settings. Existing atomic commits, kernel locks and revision conflict detection remain in effect. A failed save stays visible in the editor. Palette and appearance values arrive through snapshots, so changing the current theme does not require a sandbox remount.

## Wayland filter

The sandbox receives only the socket of a dedicated [wl-mitm](https://github.com/PeterCxy/wl-mitm) proxy, pinned to `fd1f9da658c5729c41a2dbc1c652d68115d5ca59`. Core builds this external GPL-3.0 executable with its lockfile; its source is included alongside the installation. The proxy is trusted infrastructure outside the widget sandbox and part of the security boundary.

`wayland-filter.toml` is an allowlist for drawing, outputs, ordinary input to the widget's surfaces, scaling, normal windows and constrained layer-shell surfaces. Clipboard/data-control, screen capture, virtual input, foreign-window management, session lock, DRM/GPU and unknown globals are omitted. The proxy rejects binds to hidden globals and mismatched interface names, not merely their advertisements. No raw compositor socket is mounted inside a runner.

Core's deterministic policy hook allows only bottom-layer surfaces, non-reserving exclusive zones (0 or -1), and no/on-demand keyboard input. It rejects top/overlay layers, positive exclusive zones and exclusive keyboard capture. Fullscreen requests for normal windows are rejected. Rejection is a fatal Wayland protocol error; requests that create objects are never silently dropped. The hook does not prompt the user or run package-provided commands.

Settings editors use ordinary floating windows. They do not require an overlay-layer privilege. Bottom-layer widgets stay behind application/fullscreen windows. No compositor control socket is used for automatic fullscreen detection.

## Remaining limits

This is a development security boundary, not an independently audited hostile-code platform. Linux, Qt, the compositor and the proxy remain trusted dependencies. No custom seccomp profile or network/file portal broker is supplied. Network access remains denied for every widget.

The service's `MemoryMax=512M`, `TasksMax=64` and `CPUQuota=100%` apply to the whole group. They protect the rest of the session but do not promise availability between packages: an abusive widget can exhaust this shared budget, and broker requests can delay other widgets within the bounded servicing loop. Separate per-package resource accounting remains future work.

Allowed surface protocols still let a widget create ordinary windows and bottom-layer surfaces, receive interaction with those surfaces, and misrepresent its own content. Filtering does not enforce pixel geometry, prove UI authenticity or stop social engineering. The three widget families and snap grid constrain the supported API, not arbitrary hostile Wayland clients. Instances from the same package can access one another's settings by design.

User fonts outside `/usr` are not mounted. Live Hyprland rendering, scaling, monitor hotplug, session shutdown and the World Clock settings workflow still require desktop acceptance. Wire tests against a fake compositor are protocol-enforcement evidence, not a substitute for that acceptance.

## Design sources

The shared cards, edit affordances and snap grid in [omarchy-desktop-widgets](https://github.com/cyelis1224/omarchy-desktop-widgets) informed the design; Core implements them independently. Its in-shell widget loading is not used here. Apple's widget guidance informs the fixed families; this is not WidgetKit or a claim of equivalent security.
