# GitHub contribution widget / API 3 extension

The next SDK consumer is an independently installable package at
[tcballard/omarchy-widget-github](https://github.com/tcballard/omarchy-widget-github). Working scope: personal/public-profile widget;
no marketplace submission or release is implied. The existing Weather broker
and World Clock demonstrate the runtime and settings contracts being reused.
Existing GitHub bar plugins are a different surface: this consumer exercises
Core's desktop families, instance settings, permissions and sandbox.

## Contract

- ID: `io.github.tcballard.github-contributions`; version 0.0.1.
- Manifest: `capabilities:["github"]`, `requires:["github-contributions", ...]`.
  The named feature ensures older API 3 hosts refuse installation.
- View: advanced QML `Item` with `widgetContext`; editor uses `settingsContext`.
  Declarative v1 is unchanged: no new arbitrary network or expression support.
- `widgetContext.requestGithub(username)` queues an instance-scoped request.
- `widgetContext.github`: `{state,data,updatedAt,refreshing,error}`. States:
  `loading`, `ready`, `stale`, `unavailable`. `data` is null until a valid result.
  Data: `{username,total,days:[{date,count,level,weekday}],attribution}`.
  Sunday is weekday 0; dates are provider calendar dates, never timezone-shifted.
- Username settings are per instance. A pending result for a previous username
  cannot replace the newly configured username's display.
- The manager grants/revokes access per installed immutable generation, or use
  `github-permission PACKAGE allow|deny`. Widget runners cannot self-authorise.
- Preview context supplies an unavailable result and an inert request method.

## Provider boundary

The public GitHub calendar fragment requires no login. Only validated GitHub
usernames (1–39 ASCII letters/digits/single interior hyphens) reach a fixed
`github.com/users/USERNAME/contributions` HTTPS path. No package-selected host,
URL, query, header, proxy, cookie, token or redirect is supported. Raw markup
never reaches QML. Bounded parsing validates a contiguous 365/366-day calendar,
real civil dates, weekday positions, unique cells, counts and levels 0–4.
Markup drift becomes unavailable/stale, not a successful empty calendar.

Weather's existing fixed transport/cache was extracted to `public_data.rs` and
is exercised by both providers. DNS remains public IPv4 only, pinned into curl's
TLS connection. No curl configuration or environment is inherited. The GitHub
response limit is 512 KiB (weather stays 64 KiB), HTTPS has a six-second watchdog
and DNS a two-second timeout with one-second kill grace.

Each provider retains a separate bounded cache: 128 public keys, at most 16
owned keys per package, 15-minute TTL, one fetch per package per minute, one
provider fetch per ten seconds, one-minute error retry. In-flight and late-result
handling, BOOTTIME freshness, inactive gating and generation grants remain in
force. Public results can be shared only after each reader's grant check.
No credentials or private data were added to Core's boundary. Cache is volatile.

Deferred: authenticated/private activity, streaks, repository lists, click-to-open
broker actions, wide custom widget families and declarative chart support.

## Verification (5 October 2026)

- Rust build, formatting and Clippy with `-D warnings`: passed.
- Portable Rust suite excluding four socket-restricted existing tests: 80 passed,
  two opt-in network/capture tests ignored. The full run's four existing failures
  are Unix socket creation returning `EPERM` in this environment; not passes.
- GitHub-specific parsing, grant and broker tests: passed.
- Captured real GitHub public HTML through the production parser: passed;
  366 days, total 6,729. Fixture captures below use that normalized response.
- Native PySide6 6.11.2 production QML: all families, bounds, theme change,
  loading/unavailable/stale, username validation and inactive/resume checks passed.
- Node calendar model, Core QML smoke, inert preview contract and real QML → Rust
  settings save/reopen integration: passed.
- Direct production fetch: BLOCKED here (`Provider DNS unavailable`); this
  environment's mediated curl download is not evidence for the pinned transport.
- Live Omarchy desktop, Bubblewrap and Hyprland: NOT RUN.

Reproduce portable checks:

```bash
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo fmt --all -- --check
python3 ../omarchy-widget-github/tests/qml.py .
node ../omarchy-widget-github/tests/model.cjs
python3 ../omarchy-widget-github/tests/package.py target/debug/omarchy-widget
python3 tests/preview_contract.py
```

On the XPS, first verify the actual fixed HTTP path:

```bash
cargo test --locked live_public_calendar -- --ignored --nocapture
```

Then follow the widget README installation. Check Allow/Revoke, changing username,
two instances with different users, all three sizes, hover and keyboard counts,
settings Save/Cancel/restart, theme switch, Hide/Show and workspace switching.
Disconnect networking after a successful fetch; after cache expiry the old graph
must remain visibly stale. Reconnect and verify recovery. Update the package and
verify access must be granted again. Record the tested commit and installed Core
version before declaring desktop acceptance.

## Standalone repository

The production widget, its previews and its widget-specific model/QML/package
tests live in [omarchy-widget-github](https://github.com/tcballard/omarchy-widget-github).
Core CI checks out widget commit `15f12b560eaa845b937cddd348f2f9455d36bdf4` and runs
that external consumer against the Core revision under test. Core owns the broker,
permission UI, SDK context and provider parsing tests. Its Rust broker unit test
uses a minimal generated package, so ordinary cargo tests need no remote checkout.
The widget's initial CI pins Core commit `98a2c6a599a9174123282183edad0931bcf79576`;
this is the API extension candidate, not the released v0.0.3 runtime.

After extraction, standalone package integration also passed: validation/install,
two independently saved usernames, schema rejection, hidden-instance preservation,
permission grant/revoke, update requiring re-grant, and rollback with settings kept.
The original Core PR CI run 37266374410 passed its full Rust suite (including the
four local EPERM-blocked socket tests), sandbox and resource-isolation jobs.
The revised cross-repository CI is authoritative for the extracted revision.
