# Runtime and security decision

## Implemented for v0.0.2

One standalone Quickshell process hosts all widgets, manager windows and configuration surfaces. A systemd user service provides logs, bounded restart attempts and control-group cleanup. `Service.qml` is a compatibility bridge containing no widget loader or windows. `Host.qml` runs only through the separate `shell.qml` entry point. Core owns its small theme/control implementation.

This separates widget crashes from the main shell. It does not isolate widgets from each other, and it does not restrict their access to the user's account. `NoNewPrivileges` is defence in depth, not a sandbox. The shared host can access files, network, processes and session sockets available to its user. Do not advertise marketplace-safe execution.

## Why not copy the original host architecture?

At commit `d99737791a5b983d6ba7dc6a9790e38ff4f8edfb`, [omarchy-desktop-widgets](https://github.com/cyelis1224/omarchy-desktop-widgets) registers DesktopWidgets.qml as a plugin service and loads widget components inside that service. Its shared card, edit affordances and snap grid informed the design review. Core implements those ideas independently. We do not copy its freely resizable geometry or settings writer.

Apple's [widget design guidance](https://developer.apple.com/videos/play/wwdc2020/10103/) and [desktop widget guidance](https://developer.apple.com/videos/play/wwdc2023/10027/) inform constrained families and host-owned presentation. This is not an implementation of Apple's WidgetKit or a claim of equivalent security.

## Future security boundary: a runner per package

A single sandbox around the shared host would restrict access to the computer, but every loaded widget would still share the host's capabilities and settings. Different permissions require separate runners per package (instances from one package may share a runner).

The proposed architecture keeps installation, layout and durable state in trusted Core. Each runner gets read-only code/runtime files, isolated temporary storage and only its own permitted data. Core authenticates the runner's connection to its assigned package; it must not trust a caller-supplied package ID. Settings requests carry revisions and bounded data. Runners cannot write the registry or replace packages.

Use established Linux sandbox mechanisms such as [Bubblewrap](https://github.com/containers/bubblewrap#sandbox-security), with a reviewed policy: mount/user/PID/network isolation, dropped capabilities, restricted inherited environment and descriptors, seccomp, and CPU/memory/process limits. The executable alone does not establish a security policy. Flatpak provides an existing application framework, but packaging the entire shared host as one Flatpak would still leave all its widgets sharing one boundary.

Network access should be denied by default. A trusted request broker can enforce declared services and limits. Merely sharing a network namespace does not enforce a domain allowlist: redirects, DNS, localhost/private networks and credentials all need policy. Files can be user-selected through portals where appropriate; do not expose the whole home directory.

Do not mount session D-Bus, Hyprland control sockets, SSH agents or the whole runtime directory. Wayland access is itself a capability: compositor extensions can expose privileged capture/input/session controls. A runner needs a filtered Wayland connection or a rendering architecture where trusted Core retains the compositor connection. Raw socket access is not a sufficient isolation claim.

Before calling this secure, specify the attacker model and test denied home/credential reads, writes outside instance storage, cross-package state access, process inspection/signalling, network escapes, D-Bus/compositor control and resource exhaustion. Malformed IPC must fail closed. Platform kernel and graphics-driver attack surfaces remain.

These restrictions are **not implemented in v0.0.2**. API 2's identities and acknowledged state operations are preparation, not enforcement.
