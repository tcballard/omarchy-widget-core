# Verification record

Development environment: Linux container, Python 3.12, PySide6 6.11.2, offscreen Qt. Rust compilation runs in GitHub Actions because the container has no Rust toolchain.

## Reproduced

At commit `0578a44fdd5647e883668619ca7cab976e5b4e96`, [CI run 35005950142](https://github.com/tcballard/omarchy-widget-core/actions/runs/35005950142) passed:

- 13 Rust unit tests; rustfmt check; Clippy with warnings denied.
- QML syntax and native Qt manager/frame/editor component checks.
- Production QML settings Save click → serialized queue → real Rust CLI → durable disk → reopen; stale revision rejected, placement coalescing verified, writer lock released after killing a holder process.
- Installer tests using explicit command fixtures: paths with spaces, complete payload, and Core/service/launcher restoration after activation or service-start failure. These fixtures do not claim live systemd acceptance.

After that run, local visual inspection found blank rendering when a GPU-only mask was used with Qt's software renderer. The candidate adds a software fallback and a pixel assertion; the local Qt check passes and the final branch workflow validates these changes. The source manifest beside this document identifies runtime inputs; it does not itself establish test results.

Earlier integration attempts failed on missing Python QQuickItem and QML theme imports in the harness. Those were corrected; the successful run above includes the real helper and production controller/queue/settings components. The test does not execute the Wayland window graph.

## Not run

Real Wayland/Hyprland rendering, systemd session lifecycle, crash recovery on the XPS, GPU masking across drivers, fractional scaling, monitor hotplug and live theme transitions. No installed Omarchy version is recorded as tested for this standalone runtime. See desktop-checks.md. The shared Bubblewrap policy has a dedicated denial-probe CI job; per-package and Wayland escape testing are not claimed. The local managed workspace cannot create sockets or execute the namespace probe, so local policy-plan checks are not labelled live sandbox verification.

Upstream review: Omarchy source and Quickshell CLI/process documentation were inspected during development; the original widget repository was reviewed at `d99737791a5b983d6ba7dc6a9790e38ff4f8edfb`. No substantial source from that repository was copied.

## Bubblewrap follow-up

The sandbox change adds a fourteenth Rust test for durable host controls, plus mandatory preflight and policy tests. Installer fixtures also cover a failed namespace preflight preserving the old installation. Live denial-probe results are recorded in the PR workflow; the initial attempt failed due to a socket fixture check in the test harness and was corrected. All prior results above remain historical and tied to their listed input commit.

The Ubuntu 24.04 live job failed during Bubblewrap loopback setup (`RTM_NEWADDR: Operation not permitted`), after the policy-plan checks passed. The live job was moved to Ubuntu 22.04 to exercise the same policy without weakening system security settings. This does not establish support for hosts with disabled/restricted unprivileged namespaces; those hosts fail preflight.

At runtime commit `8cfac8fb70e3725c0f62cd139567dca13f23744d`, [CI run 35014760926](https://github.com/tcballard/omarchy-widget-core/actions/runs/35014760926) passed all three jobs: Rust (14 tests, rustfmt and Clippy), QML/settings/installer checks, and the real Ubuntu 22.04 Bubblewrap denial probes. The live probe verified denied home/agent access, read-only code/package mounts, hidden host processes, denied host-network access and successful Core-state persistence. The Wayland socket was not included in the probe; live desktop and compositor-capability testing remain outstanding.
