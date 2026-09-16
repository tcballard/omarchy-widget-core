# Trusted source installation and local recovery

The installed gallery is local; there is no online catalogue. Choose a trusted
source and inspect its license, code and dependency instructions. A package ID
is a name, not authenticated ownership. Obtain an immutable revision and record
it before invoking Core, for example:

```bash
git clone https://github.com/tcballard/omarchy-widget-worldclock.git worldclock
cd worldclock
git checkout --detach REVIEWED_COMMIT_SHA
git rev-parse HEAD
omarchy-widget validate "$PWD"
bash install-local
```

Substitute a commit you have actually reviewed; do not paste the placeholder.
Core validates API compatibility before copying code. An update is another
reviewed checkout followed by `bash install-local --update`; inspect `git diff`
between the recorded commits first. Core's immutable installed generation
identifies the local copy, not source authenticity. Package source and checked-out
revision should be retained with your backup. Do not install from a directory
another process is modifying.

Use Widgets → Available to add two instances in different sizes, configure each,
then remove one. Uninstall offers keep or delete settings. Keeping settings
allows reinstall without automatically showing the retained widgets. World Clock
requires Core API 3. The countdown in `examples/countdown` is a second reference
for authors; it is not a separate released application.

## Consistent same-machine backup

Stop the host and any widget commands first. Preserve both XDG data and state,
including referenced immutable generations; `list` alone is not a backup. This
recipe is for a normal user shell with `flock`, `tar`, and the same XDG roots as
Core. Use a new destination outside both trees:

```bash
systemctl --user stop omarchy-widget-host.service
widget_state="${XDG_STATE_HOME:-$HOME/.local/state}/omarchy/widgets"
widget_data="${XDG_DATA_HOME:-$HOME/.local/share}/omarchy/widgets"
widget_backup="$HOME/widget-backup-$(date +%Y%m%d-%H%M%S)"
umask 077
mkdir "$widget_backup"
(
  flock -n 9 || exit 1
  tar -C "$widget_state" --exclude=./registry.lock -cf "$widget_backup/state.tar" . &&
  tar -C "$widget_data" -cf "$widget_backup/data.tar" . &&
  printf '%s\n%s\n' "$widget_state" "$widget_data" > "$widget_backup/roots.txt"
) 9>"$widget_state/registry.lock"
```

Check the command's exit status and both archives before restarting the service.
A partial destination is not a valid backup. Keep the service stopped while
recovering from corrupt state; preserve the corrupt files for diagnosis.

For restore, inspect trusted archives, stop Core, take the same registry lock and
restore into **new staging directories**, preserving file permissions. Save the
current directories aside before replacing them with the staged data/state pair.
Keep the original `registry.lock` inode in place throughout recovery: do not
replace or unlink it while any process can have it open. Restore to the same XDG
roots and user identity; install the matching Core build, inspect `omarchy-widget
list`, then restart the host. This is an operator recovery procedure, not a tested
one-click restore command or cross-machine transfer feature. Live backup/restore
acceptance remains a release gate. Prefer the tested `export-settings` /
`restore-settings` commands for bounded settings-only recovery.
