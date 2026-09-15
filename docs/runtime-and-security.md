# Runtime and security decision

## Implemented for v0.0.2

One standalone Quickshell process hosts all widgets, manager windows and configuration surfaces. A systemd user service provides logs, bounded restart attempts and control-group cleanup. `Service.qml` is a compatibility bridge containing no widget loader or windows. `Host.qml` runs only through the separate `shell.qml` entry point. Core owns its small theme/control implementation.

The host uses the mandatory `shared-offline-v1` Bubblewrap policy in `sandbox-launch`. Missing Bubblewrap or unavailable namespace support fails closed; the installer probes it before replacing Core. This is a shared-host sandbox, not a per-widget sandbox.

Implemented restrictions:

- Separate user, PID, IPC, network and UTS namespaces, no capabilities, a new session, private `/proc`, `/dev`, temporary storage and runtime directory.
- A cleared environment with explicitly reconstructed Qt/XDG settings. No session bus, SSH agent, X11 or Hyprland command socket is mounted or inherited.
- System runtime, Core code, installed packages and the selected theme are read-only. Only `$XDG_STATE_HOME/omarchy/widgets` is persistently writable. Temporary files remain private.
- One explicit Wayland socket is allowed. Software rendering avoids passing GPU devices into the sandbox.
- The systemd service enforces `MemoryMax=512M`, `TasksMax=64`, `CPUQuota=100%`, bounded restart attempts and control-group cleanup. These resource limits apply to the service, not a manually launched script.
- External CLI controls use the existing atomic registry and kernel lock; they do not require sharing Quickshell's session IPC directory. The host observes controls within about one second.

Limits and behaviour changes:

- All widgets still share Core's writable state. A malicious widget can alter another widget's settings or registry metadata. Installation from the external CLI remains outside this sandbox; code directories inside it are read-only.
- The raw Wayland connection is an allowed capability. Compositor extensions may permit capture or other desktop operations. It is not filtered, and this policy does not claim protection against malicious compositor-protocol use.
- Network access is denied for every widget. A domain-limited broker and per-package grants are not implemented.
- There is no custom seccomp profile. Kernel/Qt attack surfaces remain. This is not an audited hostile-code boundary.
- User-installed fonts outside `/usr` are unavailable. The theme bind is a snapshot of the selected directory; run `omarchy-widget restart` after theme-directory replacement. File changes within that directory still refresh.
- The Hyprland command socket is deliberately absent. Bottom-layer widgets stay behind fullscreen windows, but automatic fullscreen detection/background suspension is no longer advertised. Explicit hide still suspends widget content.

`tests/sandbox.py --live` uses the actual mount/namespace policy with a test payload to check denied home/credential access, package/Core writes, host process visibility and host network reachability, plus permitted durable Core state. It does not test Wayland privilege filtering, per-package separation or the live graphical host.

## Why not copy the original host architecture?

At commit `d99737791a5b983d6ba7dc6a9790e38ff4f8edfb`, [omarchy-desktop-widgets](https://github.com/cyelis1224/omarchy-desktop-widgets) registers DesktopWidgets.qml as a plugin service and loads widget components inside that service. Its shared card, edit affordances and snap grid informed the design review. Core implements those ideas independently. We do not copy its freely resizable geometry or settings writer.

Apple's [widget design guidance](https://developer.apple.com/videos/play/wwdc2020/10103/) and [desktop widget guidance](https://developer.apple.com/videos/play/wwdc2023/10027/) inform constrained families and host-owned presentation. This is not an implementation of Apple's WidgetKit or a claim of equivalent security.

## Future security boundary: a runner per package

The current shared sandbox restricts host access, but every loaded widget still shares its capabilities and settings. Different permissions require separate runners per package (instances from one package may share a runner).

The proposed architecture keeps installation, layout and durable state in trusted Core. Each runner gets read-only code/runtime files, isolated temporary storage and only its own permitted data. Core authenticates the runner's connection to its assigned package; it must not trust a caller-supplied package ID. Settings requests carry revisions and bounded data. Runners cannot write the registry or replace packages.

Use established Linux sandbox mechanisms such as [Bubblewrap](https://github.com/containers/bubblewrap#sandbox-security), with a reviewed policy: mount/user/PID/network isolation, dropped capabilities, restricted inherited environment and descriptors, seccomp, and CPU/memory/process limits. The executable alone does not establish a security policy. Flatpak provides an existing application framework, but packaging the entire shared host as one Flatpak would still leave all its widgets sharing one boundary.

Network access should be denied by default. A trusted request broker can enforce declared services and limits. Merely sharing a network namespace does not enforce a domain allowlist: redirects, DNS, localhost/private networks and credentials all need policy. Files can be user-selected through portals where appropriate; do not expose the whole home directory.

Do not mount session D-Bus, Hyprland control sockets, SSH agents or the whole runtime directory. Wayland access is itself a capability: compositor extensions can expose privileged capture/input/session controls. A runner needs a filtered Wayland connection or a rendering architecture where trusted Core retains the compositor connection. Raw socket access is not a sufficient isolation claim.

Before calling this secure, specify the attacker model and test denied home/credential reads, writes outside instance storage, cross-package state access, process inspection/signalling, network escapes, D-Bus/compositor control and resource exhaustion. Malformed IPC must fail closed. Platform kernel and graphics-driver attack surfaces remain.

The per-package runner, trusted state/network broker, filtered Wayland connection and custom seccomp policy are **not implemented**. API 2 identities prepare for them; they do not enforce them. The shared Bubblewrap policy above is the implemented first boundary.
