#!/usr/bin/env python3
"""Development-only evidence collector; Python standard library, no installation changes."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
HOST = 'omarchy-widget-host.service'
CHECKS = {
    'install': 'Clean install, shell restart, active service, launcher and bound key work; record versions.',
    'empty': 'Zero packages: capture 60 s; host CPU <=1% of one core. Record context switches separately from wakeups.',
    'single': 'One World Clock: capture 60 s; island CPU <=2% of one core. Observe add-to-first-visible-frame <=3 s.',
    'multi': 'Three packages, six instances: capture 60 s; 100 manager actions with no busy errors.',
    'hidden': 'Hide all content, capture 600 s; inspect CPU and polling cost. Record actual counts.',
    'failure': 'Broken QML disables package within 2 s; other packages survive; bounded three-attempt restart budget.',
    'settings': 'Gear, focus, Save, Cancel, WM close; edit clears. Kill owning runner while editing: recovery within 5 s.',
    'focus': 'Click passive clock from a typing application: record focus transfer and return. Gear Tab/Return and action buttons work; clicking outside restores application input. Observe actual Hyprland behaviour.',
    'contention': 'Save under sustained registry contention: measure acknowledgement/failure latency and retry notice; verify no duplicate mutation or lost draft. Repeated busy refusals may take about 12.4 s through both retry layers.',
    'arrange': 'Drag and arrow keys in all families, rejected drop, monitor cycle, Escape.',
    'topology': 'Unplug/replug preferred monitor (restore <=2 s), rotation, gaps, scales 1.25 and 1.5.',
    'reserved': 'Top/bottom/side bars: widget grid respects reserved space.',
    'workspaces': 'Workspace assignment, switching and special workspace overlay; no unintended input interception.',
    'theme': 'Light/dark recolour within 1 s; malformed widgets.json falls back; contrast and clipping.',
    'reveal': 'Default: record not-applicable after confirming disabled. Opt-in: first frame, Escape/focus, lease, lock and faulty demotion.',
    'update': 'API 2 to API 3 migration and rollback on desktop; dirty editor blocks update; later edits block destructive rollback.',
    'weather': 'Live provider request, shared refresh, revoke, disconnect and resume; renderer network remains denied.',
    'resume': 'Suspend/resume and reboot: positions persist, fresh health within 5 s. Resume this same evidence folder.',
    'lifecycle': 'Separate shell process; kill/stop/restart Core; all descendants cleaned; sibling package survives runner failure. Block stale-editor cleanup in disposable storage: recovery-blocked names the error and clears after repair.',
    'author': 'Scaffold, validate, preview, two independently configured instances, actions, migration, rollback, export/restore, uninstall.',
    'storage': 'Interrupted write and disk-full recovery in disposable storage; preserve exact fixture and results.',
    'resources': 'Disposable-user cgroup pressure tests (CI resources job); record run URL. Never stress normal desktop automatically.',
}


def write_json(path, value):
    tmp = path.with_suffix(path.suffix + '.tmp')
    tmp.write_text(json.dumps(value, indent=2) + '\n')
    tmp.replace(path)


def command(argv, timeout=30, env=None):
    """Kill the entire test process group on timeout, including spawned fixtures."""
    import tempfile
    started = time.monotonic()
    with tempfile.TemporaryFile() as output:
        try:
            process = subprocess.Popen(argv, cwd=ROOT, stdout=output, stderr=subprocess.STDOUT,
                                       stdin=subprocess.DEVNULL, start_new_session=True, env=env)
        except OSError as error:
            return {'argv': argv, 'status': 'blocked', 'output': str(error)}
        try:
            code = process.wait(timeout=timeout)
            status = 'pass' if code == 0 else 'fail'
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait()
            code, status = None, 'timeout'
        except BaseException:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait()
            raise
        output.seek(0)
        return {'argv': argv, 'status': status, 'exit_code': code,
                'elapsed_seconds': time.monotonic() - started,
                'output': output.read().decode(errors='replace')}


def manifest(path):
    """Logical and allocated bytes; skip symlinks and count hardlinks only once."""
    if not path.exists():
        return {'status': 'missing', 'path': str(path)}
    files, seen, errors = [], set(), []
    candidates = [path] if path.is_file() else path.rglob('*')
    for item in candidates:
        try:
            if item.is_symlink() or not item.is_file():
                continue
            stat = item.stat()
            identity = (stat.st_dev, stat.st_ino)
            if identity in seen:
                continue
            seen.add(identity)
            digest = hashlib.sha256()
            with item.open('rb') as stream:
                for chunk in iter(lambda: stream.read(1024 * 1024), b''):
                    digest.update(chunk)
            files.append({'path': str(item.relative_to(path)) if item != path else item.name,
                          'bytes': stat.st_size, 'allocated_bytes': stat.st_blocks * 512,
                          'sha256': digest.hexdigest()})
        except OSError as error:
            errors.append(str(error))
    return {'status': 'partial' if errors else 'measured', 'path': str(path), 'files': files,
            'bytes': sum(f['bytes'] for f in files),
            'allocated_bytes': sum(f['allocated_bytes'] for f in files), 'errors': errors}


def units():
    result = command(['systemctl', '--user', 'list-units', '--all', '--plain', '--no-legend',
                      '--no-pager', HOST, 'omarchy-widget-island-*.service'])
    if result['status'] != 'pass':
        raise RuntimeError(result['output'])
    names = {line.split()[0] for line in result['output'].splitlines() if line.split()}
    groups = {}
    for name in sorted(names):
        if name != HOST and not (name.startswith('omarchy-widget-island-') and name.endswith('.service')):
            continue
        result = command(['systemctl', '--user', 'show', name, '--property=ControlGroup', '--value'])
        relative = result['output'].strip()
        if result['status'] != 'pass':
            raise RuntimeError(result['output'])
        if relative:
            group = Path('/sys/fs/cgroup') / relative.lstrip('/')
            if not group.resolve().is_relative_to('/sys/fs/cgroup'):
                raise RuntimeError('Unsafe cgroup path')
            groups[name] = group
    if HOST not in groups:
        raise RuntimeError('Core host has no active cgroup; start the installed host first')
    for a in groups.values():
        for b in groups.values():
            if a != b and a.is_relative_to(b):
                raise RuntimeError('Overlapping cgroups would double-count resources')
    return groups


def sample_group(group):
    stats = dict(line.split() for line in (group / 'cpu.stat').read_text().splitlines())
    pids = set()
    for file in group.rglob('cgroup.procs'):
        pids.update(file.read_text().split())
    processes, errors = [], []
    for pid in sorted(pids, key=int):
        try:
            base = Path('/proc') / pid
            # Field 22 starts after the final ')' of comm, which may itself contain spaces.
            identity = base.joinpath('stat').read_text().rsplit(')', 1)[1].split()[19]
            values = {}
            for line in base.joinpath('smaps_rollup').read_text().splitlines():
                if line.startswith(('Rss:', 'Pss:')):
                    values[line.split(':')[0].lower() + '_bytes'] = int(line.split()[1]) * 1024
            switches = sum(int(line.split()[1]) for line in base.joinpath('status').read_text().splitlines()
                           if line.startswith(('voluntary_ctxt_switches:', 'nonvoluntary_ctxt_switches:')))
            processes.append({'pid': int(pid), 'start_ticks': identity, 'name': base.joinpath('comm').read_text().strip(),
                              'context_switches': switches, **values})
        except (OSError, ValueError, IndexError) as error:
            errors.append(f'pid {pid}: {error}')
    return {'identity': group.stat().st_ino, 'cpu_usec': int(stats['usage_usec']),
            'memory_current_bytes': int((group / 'memory.current').read_text()),
            'tasks': int((group / 'pids.current').read_text()), 'process_count': len(pids),
            'processes': processes, 'process_errors': errors,
            'pss_bytes': None if errors or any('pss_bytes' not in p for p in processes) else sum(p['pss_bytes'] for p in processes)}


def cpu_percent(before, after, seconds):
    if before['identity'] != after['identity'] or after['cpu_usec'] < before['cpu_usec']:
        return None
    return (after['cpu_usec'] - before['cpu_usec']) / (seconds * 10000)


def capture(seconds, on_sample=None):
    samples, errors = [], []
    start = time.monotonic()
    expected = None
    while True:
        row = {'elapsed_seconds': time.monotonic() - start, 'units': {}}
        try:
            groups = units()
            if expected is None:
                expected = set(groups)
            if set(groups) != expected:
                errors.append('Unit set changed during capture; repeat steady-state measurement')
            for name, path in groups.items():
                row['units'][name] = sample_group(path)
        except (OSError, ValueError, RuntimeError) as error:
            errors.append(str(error))
        row['elapsed_seconds'] = time.monotonic() - start
        samples.append(row)
        if on_sample:
            on_sample(row)
        if errors or row['elapsed_seconds'] >= seconds:
            break
        time.sleep(min(1, seconds - row['elapsed_seconds']))
    summary = {}
    if len(samples) >= 2 and not errors:
        duration = samples[-1]['elapsed_seconds'] - samples[0]['elapsed_seconds']
        for name in sorted(expected):
            rows = [s['units'][name] for s in samples]
            cpu = cpu_percent(rows[0], rows[-1], duration)
            if cpu is None or any(r['identity'] != rows[0]['identity'] for r in rows):
                errors.append(f'{name}: cgroup restarted; measurement invalid')
            pss = [r['pss_bytes'] for r in rows]
            summary[name] = {'cpu_percent_one_core': cpu,
                             'mean_memory_current_mib': sum(r['memory_current_bytes'] for r in rows) / len(rows) / 2**20,
                             'max_sampled_memory_current_mib': max(r['memory_current_bytes'] for r in rows) / 2**20,
                             'mean_pss_mib': None if None in pss else sum(pss) / len(pss) / 2**20,
                             'max_sampled_process_count': max(r['process_count'] for r in rows)}
    return {'status': 'blocked' if errors else 'measured', 'requested_seconds': seconds,
            'errors': errors, 'summary': summary, 'samples': samples,
            'note': 'No automatic acceptance. CPU is percent of one core. memory.current includes cache; PSS apportions shared pages. Sampled peaks miss between-sample spikes. Context switches are not wakeups. Short-lived processes may be absent from /proc samples; cgroup CPU includes them.'}


def suite(worldclock=None, older_core=None, proxy=None):
    helper = str(ROOT / 'target/debug/omarchy-widget')
    jobs = [('harness', [sys.executable, 'tests/desktop_review.py']),
            ('format', ['cargo', 'fmt', '--all', '--', '--check']),
            ('rust-default', ['cargo', 'test', '--locked']),
            ('rust-reveal', ['cargo', 'test', '--locked', '--features', 'experimental-reveal']),
            ('clippy', ['cargo', 'clippy', '--locked', '--all-targets', '--', '-D', 'warnings']),
            ('build', ['cargo', 'build', '--locked']),
            ('release-size-build', ['cargo', 'build', '--release', '--locked'])]
    no_helper = ['qml_smoke', 'lifecycle', 'window_lifecycle', 'preview_contract', 'installer', 'keybinding', 'sandbox']
    with_helper = ['declarative', 'declarative_controller', 'sdk', 'review_delivery', 'package_helpers', 'countdown_integration', 'runtime_integration']
    jobs += [(name, [sys.executable, f'tests/{name}.py']) for name in no_helper]
    jobs += [(name, [sys.executable, f'tests/{name}.py', helper]) for name in with_helper]
    jobs += [(f'manager-{scale}-{repeat}', ['env', f'QT_SCALE_FACTOR={scale}', sys.executable,
                                          'tests/manager_integration.py', helper])
             for scale in ['1', '1.25', '1.5', '2'] for repeat in range(1, 11)]
    jobs += [(name, ['node', f'tests/{name}.cjs']) for name in ['workspace', 'grid']]
    jobs += [(f'shell-{name}', ['bash', '-n', name]) for name in
             ['install-local', 'widget', 'host-launch', 'sandbox-launch', 'bind-key']]
    jobs += [('sandbox-live', [sys.executable, 'tests/sandbox.py', '--live']),
             ('preview-sandbox', ['/usr/bin/python3', 'tests/preview_contract.py', '--sandbox'])]
    jobs += [('worldclock', [sys.executable, 'tests/worldclock_integration.py', helper, str(worldclock)] if worldclock else None),
             ('clock-previews', [sys.executable, 'sdk/render_previews.py', str(OUT / 'clock-previews'), '--package', str(worldclock)] if worldclock else None),
             ('clock-sandbox-previews', ['bash', 'sdk/capture-package', str(worldclock), str(OUT / 'clock-sandbox-previews')] if worldclock else None),
             ('api-compatibility', [sys.executable, 'tests/api_compatibility.py', helper, str(older_core)] if older_core else None),
             ('wayland-default', [sys.executable, 'tests/wayland_filter.py', str(proxy), helper] if proxy else None)]
    built = False
    for name, argv in jobs:
        if argv and helper in argv and not built:
            result = {'status': 'blocked', 'output': 'Default build failed; refusing to test a stale helper.'}
        else:
            result = command(argv, timeout=900) if argv else {'status': 'blocked', 'output': 'Supply the documented fixture argument; not run.'}
        if name == 'build':
            built = result['status'] == 'pass'
        yield name, result
    if proxy and built:
        result = command(['cargo', 'build', '--locked', '--features', 'experimental-reveal'], timeout=900)
        yield 'build-reveal', result
        if result['status'] == 'pass':
            yield 'wayland-reveal', command([sys.executable, 'tests/wayland_filter.py', str(proxy), helper, '--experimental-reveal'], timeout=900)
        yield 'restore-default-build', command(['cargo', 'build', '--locked'], timeout=900)
    else:
        yield 'wayland-reveal', {'status': 'blocked', 'output': 'Supply --proxy; not run.'}
    yield 'resources-live', {'status': 'not-run', 'output': 'Run existing resources CI job in its disposable user; record URL under resources. Never run on the normal desktop.'}


def report(out):
    checks = json.loads((out / 'checks.json').read_text())
    lines = ['# Widget Core evidence', '', 'No aggregate pass: portable tests, measurements and human observations have different boundaries.', '',
             '## Desktop observations', '', '| Check | Status | Observation |', '| --- | --- | --- |']
    for key, value in checks.items():
        note = value.get('note', '').replace('|', '\\|').replace('\n', '<br>')
        lines.append(f"| {key} | {value['status']} | {note} |")
    lines += ['', '## Captures and tests', '']
    for file in sorted(out.glob('event-*.json')):
        value = json.loads(file.read_text())
        lines.append(f"- {value['name']}: **{value['result']['status']}** — [{file.name}]({file.name})")
        for name, stats in value['result'].get('summary', {}).items():
            lines.append(f"  - {name}: {json.dumps(stats)}")
    lines += ['', '## Footprint', '', 'MiB = 1,048,576 bytes; MB = 1,000,000 bytes. Payload totals overlap individual binaries; do not add them.', '']
    for file in sorted(out.glob('footprint-*.json')):
        values = json.loads(file.read_text())
        for name, value in values.items():
            size = f"{value['bytes'] / 2**20:.2f} MiB ({value['bytes'] / 1e6:.2f} MB)" if 'bytes' in value else value['status']
            lines.append(f'- {file.name} / {name}: {size}')
    (out / 'report.md').write_text('\n'.join(lines) + '\n')


def event(out, name, result):
    write_json(out / f'event-{time.time_ns()}.json', {'name': name, 'unix_time': time.time(), 'result': result})
    report(out)


def initialize(out):
    out.mkdir(parents=True, exist_ok=False, mode=0o700)
    write_json(out / 'checks.json', {k: {'status': 'not-run', 'instructions': v} for k, v in CHECKS.items()})
    (out / 'checklist.md').write_text('# Desktop checks\n\n' + '\n\n'.join(f'## {k}\n\n{v}' for k, v in CHECKS.items()) + '\n')
    report(out)


def environment(out, core_dir):
    probes = {
        'source': ['git', 'rev-parse', 'HEAD'], 'source-changes': ['git', 'status', '--porcelain'],
        'kernel': ['uname', '-a'], 'omarchy': ['omarchy-version'], 'hyprland': ['hyprctl', 'version'],
        'monitors': ['hyprctl', '-j', 'monitors'], 'quickshell': ['qs', '--version'],
        'packages': ['pacman', '-Qi', 'quickshell', 'qt6-base', 'qt6-declarative', 'bubblewrap'],
        'service': ['systemctl', '--user', 'show', HOST, '--property=ActiveState,SubState,ExecStart,ControlGroup'],
        'rust': ['rustc', '--version'], 'python': [sys.executable, '--version'],
    }
    for name, argv in probes.items():
        event(out, f'environment-{name}', command(argv))
    write_json(out / f'footprint-{time.time_ns()}.json', {
        'installed-core-payload': manifest(core_dir),
        'installed-rust-binary': manifest(core_dir / 'bin/omarchy-widget'),
        'installed-wayland-proxy': manifest(core_dir / 'bin/wl-mitm'),
        'checkout-release-binary': manifest(ROOT / 'target/release/omarchy-widget'),
    })
    report(out)


def main():
    global OUT
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True, help='Private evidence folder; reuse for subsequent steps')
    sub = parser.add_subparsers(dest='action', required=True)
    for action in ['start', 'environment']:
        p = sub.add_parser(action)
        p.add_argument('--core-dir', type=Path, default=Path.home() / '.config/omarchy/plugins/io.github.tcballard.widget-core')
    p = sub.add_parser('suite')
    p.add_argument('--worldclock', type=Path)
    p.add_argument('--older-core', type=Path, help='Built main dc96dd4 helper binary')
    p.add_argument('--proxy', type=Path, help='Built pinned wl-mitm binary')
    p = sub.add_parser('capture')
    p.add_argument('label', choices=['empty', 'single', 'multi', 'hidden', 'custom'])
    p.add_argument('--seconds', type=int, default=60)
    p.add_argument('--note', required=True, help='Actual package/instance counts, visible/hidden state and activity')
    p = sub.add_parser('record')
    p.add_argument('check', choices=CHECKS)
    p.add_argument('status', choices=['pass', 'fail', 'blocked', 'not-applicable'])
    p.add_argument('--note', required=True, help='Exact observation and evidence filename or CI URL; required even for skips')
    sub.add_parser('report')
    args = parser.parse_args()
    os.umask(0o077)
    OUT = args.output.expanduser().resolve()
    if args.action == 'start':
        initialize(OUT)
    elif not (OUT / 'checks.json').is_file():
        parser.error('Run start first; the evidence folder is not initialized')
    if args.action in ['start', 'environment']:
        environment(OUT, args.core_dir.expanduser().resolve())
    elif args.action == 'suite':
        failed = False
        for key in ['worldclock', 'older_core', 'proxy']:
            fixture = getattr(args, key)
            if fixture:
                event(OUT, f'fixture-{key}', {'status': 'recorded', 'manifest': manifest(fixture.resolve()) if fixture.is_file() else None,
                      'revision': command(['git', '-C', str(fixture.resolve()), 'rev-parse', 'HEAD']) if fixture.is_dir() else None,
                      'changes': command(['git', '-C', str(fixture.resolve()), 'status', '--porcelain']) if fixture.is_dir() else None})
        for name, result in suite(*(getattr(args, key).resolve() if getattr(args, key) else None
                                    for key in ['worldclock', 'older_core', 'proxy'])):
            print(name + ': ' + result['status'], flush=True)
            event(OUT, name, result)
            failed |= result['status'] != 'pass'
        environment(OUT, Path.home() / '.config/omarchy/plugins/io.github.tcballard.widget-core')
        return int(failed)
    elif args.action == 'capture':
        if not 1 <= args.seconds <= 3600:
            parser.error('--seconds must be between 1 and 3600')
        event(OUT, 'capture-source', command(['git', 'rev-parse', 'HEAD']))
        event(OUT, 'capture-source-changes', command(['git', 'status', '--porcelain']))
        stream_path = OUT / f'samples-{time.time_ns()}.jsonl'
        with stream_path.open('w') as stream:
            def persist(row):
                stream.write(json.dumps(row) + '\n')
                stream.flush()
            result = capture(args.seconds, persist)
        result['stream_file'] = stream_path.name
        result['scenario_note'] = args.note
        event(OUT, f'capture-{args.label}', result)
        event(OUT, 'capture-journal', command(['journalctl', '--user', '-u', HOST, '-u', 'omarchy-widget-island-*.service', '--since', f'{args.seconds + 60} seconds ago', '--no-pager', '-n', '2000']))
        return int(result['status'] != 'measured')
    elif args.action == 'record':
        if not args.note.strip():
            parser.error('--note cannot be empty')
        checks = json.loads((OUT / 'checks.json').read_text())
        checks[args.check].update(status=args.status, note=args.note, unix_time=time.time())
        write_json(OUT / 'checks.json', checks)
        event(OUT, f'observation-{args.check}', checks[args.check])
    report(OUT)
    print(OUT / 'report.md')
    return 0


if __name__ == '__main__':
    try:
        sys.exit(main())
    except (OSError, RuntimeError) as error:
        sys.exit(str(error))
