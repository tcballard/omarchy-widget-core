# Runtime and security

## Widget islands (v0.0.2 development)

Trusted Core owns installation, the registry, durable settings, layout, themes and the manager. The main Omarchy shell loads only the compatibility bridge. A Rust supervisor starts the trusted manager on demand for Widgets, arrange mode or experimental reveal, and a separate systemd user service for each active package. The manager exits after its trusted surfaces close and queued operations finish; package settings remain owned by their runner. Manager launch retries are bounded to three attempts per open request with a five-second delay. Close and reopen Widgets to reset a failed request. That package service contains a worker, its Wayland proxy, policy hooks, Bubblewrap runner and all their descendants. Instances belonging to the same package share a runner and a trust boundary. Widget views and settings editors are never loaded by the manager.

A runner is started for an enabled instance or an outstanding settings request. Replacing its package generation stops the old runner; the new runner loads the new immutable code. A runner or filter failure stops that island and allows up to three launch/failure attempts per Core session, with a five-second delay. Restart that package in Widgets to reset this budget, or use `package-control PACKAGE restart`. New code generations also reset it. Failures appear in the manager and logs.

Each runner has mandatory Bubblewrap isolation: separate user, PID, IPC, network and UTS namespaces; dropped capabilities; a cleared environment; private temporary storage, `/proc` and `/dev`; and read-only system/Core runtime files. It sees exactly one package at `/widget`. It has **no mount of the registry, another package, home directory, theme directory, host runtime directory, D-Bus, SSH agent, X11 socket, Hyprland command socket or GPU devices**. Software rendering is required. There is no unsandboxed fallback.

## State broker

The supervisor creates a different private Unix listener for each package generation. Only that listener is mounted into that package's sandbox. Authority comes from the supervisor's listener association, never a package ID or path supplied in a message. Standard-library descriptors are close-on-exec; other listeners are not inherited by runner processes.

The broker accepts bounded JSON argument arrays (16 KiB), with read/write deadlines and a fixed per-loop connection budget. Its client caps responses at 2 MiB. It allows a filtered snapshot, revision-checked settings saves, placement changes, hiding owned instances and acknowledgement of owned settings requests. It also permits entering/leaving the shared arrangement UI, owned settings-window registration and authorised weather requests. It rejects installation, removal, arbitrary registry operations, legacy unconditional configuration and cross-package instance IDs. An old code generation loses authority when its registry pointer changes.

Core is the only writer of durable settings. Existing atomic commits, kernel locks and revision conflict detection remain in effect. A failed save stays visible in the editor. Palette and appearance values arrive through snapshots, so changing the current theme does not require a sandbox remount.

## Wayland filter

The sandbox receives only the socket of a dedicated [wl-mitm](https://github.com/PeterCxy/wl-mitm) proxy, pinned to `fd1f9da658c5729c41a2dbc1c652d68115d5ca59`. Core builds this external GPL-3.0 executable with its lockfile; its source is included alongside the installation. The proxy is trusted infrastructure outside the widget sandbox and part of the security boundary.

`wayland-filter.toml` is an allowlist for drawing, outputs, ordinary input to the widget's surfaces, scaling, normal windows and constrained layer-shell surfaces. Clipboard/data-control, screen capture, virtual input, foreign-window management, session lock, DRM/GPU and unknown globals are omitted. The proxy rejects binds to hidden globals and mismatched interface names, not merely their advertisements. No raw compositor socket is mounted inside a runner.

Core's deterministic policy hook normally allows only bottom-layer surfaces, non-reserving exclusive zones (0 or -1), and no/on-demand keyboard input. A short Core-owned reveal lease additionally permits overlay surfaces; ending it stops the whole runner service. Top layers, positive exclusive zones and exclusive keyboard capture stay denied. Fullscreen requests for normal windows are rejected. Rejection is a fatal Wayland protocol error; requests that create objects are never silently dropped. The hook does not prompt the user or run package-provided commands.

Settings editors use ordinary floating windows. They do not require an overlay-layer privilege. Bottom-layer widgets stay behind application/fullscreen windows. No compositor control socket is used for automatic fullscreen detection.

## Remaining limits

This is a development security boundary, not an independently audited hostile-code platform. Linux, Qt, the compositor and the proxy remain trusted dependencies. No custom seccomp profile or general network/file portal is supplied. Renderer network access remains denied. The optional fixed-provider weather broker is documented in network-widgets.md.

Each package service has its own enforced budget: `MemoryMax=256M` (256 MiB), `MemorySwapMax=0`, `CPUQuota=25%` (one quarter of one CPU), and `TasksMax=64` (processes and threads combined). Instances from the same package share this budget. These are ceilings, not reserved allocations or measured typical consumption. CPU pressure throttles the package; the task ceiling rejects further task creation. `OOMPolicy=kill` makes a cgroup memory-exhaustion event terminate the entire offending package, including its proxy. The supervisor applies its existing three-attempt session budget and five-second retry delay.

Core's manager, broker and systemd-run wait clients stay in `omarchy-widget-host.service`, with their own `MemoryMax=512M`, `CPUQuota=100%` and `TasksMax=256`. Package services are sibling cgroups in `app.slice`, not children of Core's memory/CPU budget. Core's task allowance accommodates wait clients for the supported package count; it is not granted to any widget. `BindsTo` and `After` stop package services when Core stops. `PartOf` is deliberately absent: propagating a host restart would relaunch stale runners alongside the new supervisor's generation. Only the supervisor creates replacement runners. Explicit package teardown stops the transient unit with `KillMode=control-group`; killing a systemd-run client alone is not treated as cleanup. `omarchy-widget logs` includes package-unit logs.

Before executing the proxy or widget code, the worker reads its actual cgroup v2 controller files and rejects absent, unlimited or weaker CPU/memory/task limits, nonzero swap allowance or missing group OOM handling. The installer runs the same checks in a temporary constrained service before replacing Core, without starting the installed Core service. There is no unrestricted fallback. This requires systemd 254+ and CPU, memory and pids controllers available to the user manager. The installer does not edit system-wide controller delegation. Defaults are Core-owned; widget manifests cannot raise them.

This isolates a package's resource exhaustion from sibling package budgets. It is not a total desktop resource reservation: aggregate demand from many widgets, tighter ancestor cgroup limits, host-wide OOM and shared broker work can still affect availability. Broker servicing remains bounded by message sizes and absolute I/O deadlines. Hardware acceptance and budget tuning on real Qt/Quickshell builds are still required.

Allowed surface protocols still let a widget create ordinary windows and bottom-layer surfaces, receive interaction with those surfaces, and misrepresent its own content. Filtering does not enforce pixel geometry, prove UI authenticity or stop social engineering. The three widget families and snap grid constrain the supported API, not arbitrary hostile Wayland clients. Instances from the same package can access one another's settings by design.

User fonts outside `/usr` are not mounted. Live Hyprland rendering, scaling, monitor hotplug, session shutdown and the World Clock settings workflow still require desktop acceptance. Wire tests against a fake compositor are protocol-enforcement evidence, not a substitute for that acceptance.

## Design sources

The shared cards, edit affordances and snap grid in [omarchy-desktop-widgets](https://github.com/cyelis1224/omarchy-desktop-widgets) informed the design; Core implements them independently. Its in-shell widget loading is not used here. Apple's widget guidance informs the fixed families; this is not WidgetKit or a claim of equivalent security.

## Workspace tracking extension

Trusted Core now performs a fixed `j/monitors` query through Hyprland's Unix control socket. It imports `HYPRLAND_INSTANCE_SIGNATURE` alongside the existing desktop routing variables. Widget sandboxes still receive no control socket or session signature. Bounded output names, active workspace IDs, usable cell grids, frame metrics and anonymous occupancy are included in snapshots; query responses have a 64 KiB cap, a 100 ms read deadline and a 250 ms shared cache. Four fixed getoption requests read global gaps, border size and rounding. Missing/malformed/unavailable desktop data leaves all widgets unplaced. This does not add fullscreen detection or expand allowed Wayland protocols.

The workspace restriction is enforced by the cooperative Core host's visibility/Loader logic, not by the Wayland proxy. A hostile package is not prevented from creating another permitted bottom-layer surface. The broker separately prevents cross-package assignment changes.

## Application boundary and combined-review changes

Core is an external supervised desktop application. Its optional shell plugin is
only a bridge; it does not load widget QML. This deliberate process separation
supports per-package containment and differs from Omarchy's ordinary in-shell
plugin guidance. The development install layout is not proof of marketplace or
upstream acceptance. See the README for scope and remaining packaging work.

Package mutations retain atomic generation authorization. Shared-lock reads and
bounded pre-dispatch busy retries reduce contention without replaying uncertain
writes. Five mutation attempts per second per runner bound broker write churn;
packages cannot enter global arrangement. Ordinary widget surfaces use on-demand
keyboard focus, never exclusive capture. Reveal privileges are disabled in the
default build; only the explicit experimental feature can enable the prototype.

Weather fetches have a 30-second pending deadline in the suspend-aware clock domain.
Expired work becomes retryable/evictable. A fetch serial prevents a late completion
from overwriting a newer request or a re-created cache entry. This remains one
fixed provider, not a generic networking service or provider framework.

The supervisor waits on broker socket readiness instead of waking every 20 ms. One-second housekeeping still discovers registry, desktop and runner-liveness changes; broker connections wake it immediately. Runner snapshot polling remains at one second to preserve workspace/theme response. A closed manager contributes no recurring snapshot poll. Fully event-driven desktop/theme updates are not claimed.

The September 17 live baseline measured one World Clock plus a closed resident manager at 237.4 MiB mean combined PSS and 2.39% of one core. The manager Quickshell process accounted for 134.4 MiB PSS. After the manager lifecycle and supervisor-wait changes, the same desktop scenario at source `90fae72` measured 90.6 MiB combined mean PSS and 1.58% of one core: reductions of 146.7 MiB (61.8%) and 0.80 percentage points (33.7%). The host fell to 1.9 MiB PSS and 0.44% CPU; the World Clock island measured 88.7 MiB PSS and 1.14% CPU. Exactly one current-generation island remained throughout 58 samples, the manager process was absent, and no visibility binding warning occurred during the capture. These figures cover one machine and one 60-second steady-state trial; multi-package scaling still requires measurement.
