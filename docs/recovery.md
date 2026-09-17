# Updates, settings and package faults

A package may declare `settingsVersion` (positive integer, default 1),
`settingsSchema` and `migrations`. Schema support is deliberately bounded:
object, array, string, number, integer, boolean; properties, required,
additionalProperties, items, maxItems, maxLength, minimum, maximum and enum.
Unknown schema keywords are rejected. This is a subset, not general JSON Schema.
Defaults, saved settings, migrated settings and rollback settings are validated.

Example version 2 migration:

```json
{"settingsVersion":2,"migrations":[{"from":1,"to":2,"rename":{"city":"label"},"defaults":{"format":"24h"}}]}
```

Each step advances exactly one version. Operations rename top-level fields,
add absent defaults or remove explicitly listed keys. Rename collisions fail;
Core never runs package migration scripts. Missing steps or invalid results
leave the active version and every instance unchanged. Source changes during
staging also fail before pointer activation.

The atomic registry commit includes the immutable code pointer, migrated settings
and before/after settings checkpoints. Identity, placement, workspace and hidden
state are preserved. Settings revisions advance when migration changes data or
schema, invalidating old editors. One previous code/settings checkpoint is kept;
the existing 1-MiB registry cap still applies, so a large checkpoint may cause an
update to be rejected without changing the old registry. Orphan version files
are harmless and not automatically deleted.

`rollback PACKAGE` (also in Widgets → Available) checks family compatibility and
restores the matching settings checkpoint when the schema changed. Later moves,
hides and instance removals survive. Later settings edits or newly added instances
with an incompatible schema cause rollback to fail with an explanation; it never
silently overwrites them. Use Available → Export settings (or `export-settings PACKAGE`) and install a forward
compatible package in that case. A same-schema rollback keeps compatible edits.
Rollback can be reversed again using the swapped checkpoint. Weather permission
is revoked; code rollback does not revive an old grant.

## Fault controls

The Available tab reports running, retrying, failed, disabled or idle package
runners. Health expires after five seconds without the supervisor heartbeat.
`package-control PACKAGE restart|disable|enable` is trusted-manager-only. Restart
resets that package's retry budget; disable stops its runner while preserving all
instances and their enabled preferences. New code generations reset failed starts.
Three failures stop automatic retries, with five seconds between attempts.

Containment remains **per package**, including all its instances, proxy and child
processes. The existing systemd cgroup enforces CPU, memory and task limits.
A bad instance can disrupt siblings in its package; unrelated package runners
are separate. The trusted manager, compositor and OS remain shared dependencies.
A QML load error is shown within the widget and reported through a retained, generation-bound message; runner health alone is process health,
not a guarantee that every content component loaded successfully.

Portable tests inject missing migrations, rename conflicts, schema-invalid saves,
and rollback-after-edit conflicts, and verify unchanged registry state. They also
exercise checkpoint reversal, visibility preservation and package control serials.
Actual runaway-process resource containment remains covered by the existing
resource CI job. Live desktop crash/restart presentation remains unverified.


Final integration: every gear/manager settings request registers with Core before
loading. Updates and rollback refuse while that package has an open editor.
An explicit package restart/disable clears a stuck request and discards volatile
drafts; use Save/Cancel first when the process is healthy.

## Supported settings export and restore

`omarchy-widget export-settings PACKAGE` takes the registry lock and writes a
private JSON file under the state directory's `exports/`, returning its full path.
It records package/generation/revision and all per-instance settings checkpoints.
The manager's Available tab exposes this action and displays the saved path.

`omarchy-widget restore-settings PACKAGE /absolute/path/to/export.json` is an
explicit replacement of settings for matching existing instance IDs. Close any
settings editor first. Every imported setting is migrated/validated against the
currently installed package under the same lock. One invalid setting, foreign
existing identity or incompatible migration rejects changes to all matching
instances. Removed identities are skipped and listed in the response; they are
never recreated. A file with no matching instances is refused. Revisions advance,
while positions and hidden state remain unchanged. Save another export
before restoring edits you may want to retain. This is not cross-machine import.

## Failure outcomes

No automatic rollback is attempted after an author changes settings. Staging or
precommit failure leaves the old code/settings pair; a failure after the atomic
commit leaves the new pair. Retry `list` after an uncertain acknowledgement.
The supervisor replaces the runner separately. Process failure follows the existing
three-attempt budget. A content Loader error reports through its scoped broker,
disables only that package and makes the manager show a stopped-content error.
Settings stay available for export. Roll back or install a compatible version,
then Enable package. A process that starts is not proof of healthy content.

Restart/disable clears a crashed editor registration; it deliberately discards
unacknowledged drafts. There is no automatic draft recovery promise. Regression
tests cover injected staging/precommit/postcommit errors, migration refusal,
restore refusal and explicit failed-content state. Actual power loss, disk-full,
first-load presentation and supervisor replacement still require desktop tests.

## Interrupted settings and damaged layout

Closing settings retains its instance/serial acknowledgement until accepted or a
snapshot confirms that registration is absent. The UI retries delivery after a
full queue or transient refusal. A stale close never clears a later editor. The
supervisor clears dead-runner registrations; Widgets also names the pending
instance and offers Cancel pending settings. Restart in Available remains a remedy.

`omarchy-widget repair` quarantines individually invalid placements and malformed
runtime controls. The manager displays the valid subset without writing the file,
shows a repair banner, and offers the same action. Repair first saves an exact,
private `layout-quarantine-*.json` backup, then atomically commits the repaired
layout. Retired identities are not reused. Unsupported versions, invalid package
metadata and unparseable JSON require a matched backup; they are not guessed or
silently reset. A read failure no longer terminates the supervisor and its current
runners. Full disk/power-loss and real-desktop recovery still need acceptance.
