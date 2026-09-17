# Quick reveal

**Disabled by default.** This prototype requires the explicit `experimental-reveal` Cargo feature. The normal install does not enable it. Runner restarts, latency, focus restoration, lock and suspend remain desktop acceptance work.

`omarchy-widget reveal` toggles a 30-second glance at enabled widgets on their
assigned workspaces. `omarchy-widget dismiss-reveal` ends it. Escape dismisses
from Core's trusted keyboard surface. The manager remains a separate action.
Bind the command to a free key in your personal Hyprland configuration, e.g.
`bindd = SUPER ALT, R, Reveal widgets, exec, omarchy-widget reveal` (conf format),
or `o.bind("SUPER + ALT + R", "Reveal widgets", "omarchy-widget reveal")` (Lua).
Check for an existing binding first; Core does not overwrite it.

Application windows receive no move, resize, workspace or focus commands. Their
placement is unchanged. Compositor focus restoration still needs live acceptance.
Saved instance enablement, global visibility, cells and settings are untouched.
Hidden instances stay hidden. Finish settings before reveal; settings entry is
unavailable during the glance.

## Authority and expiry

The supervisor creates a private lease outside the sandbox and restarts package
runners when entering/leaving reveal. Only a leased proxy accepts overlay layer
requests. Exclusive keyboard capture, exclusive zones, fullscreen and forbidden
globals remain denied. A trusted worker uses a monotonic deadline to terminate
proxy and sandbox at expiry even if the supervisor stalls. Ending reveal stops
the whole package service, removing existing elevated surfaces rather than
trusting widget code to demote them. Volatile widget state must be reconstructible
from settings or absolute timestamps; background process identity is not stable.

Session lock protection is the compositor's responsibility: the
[session-lock protocol](https://wayland.app/protocols/ext-session-lock-v1)
requires normal client content and input to be hidden while locked. Core grants
no lock-surface protocol. This is not a tested Hyprland lock-screen guarantee.

## Evidence and remaining acceptance

Portable Rust tests cover lease bounds, privilege denial, toggle semantics and
layout preservation. The actual-proxy CI test now covers leased overlay acceptance
and revocation for new requests. QML smoke passes. Live acceptance must cover
multiple outputs, fullscreen apps, Escape, focus return, locking during reveal,
expiry and a malicious runner retaining its surface. This PR is a development
prototype until those tests are recorded. A reveal restarts renderers and may
briefly flash while they initialise.
