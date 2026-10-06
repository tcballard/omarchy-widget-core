#!/usr/bin/env python3
"""Non-destructive live worker lifetime regression; only owns unique test units.
Uses the production worker and cgroup policy with inert proxy/payload fixtures.
Never starts/stops the installed Core service or runs resource-pressure workloads.
"""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time

helper = Path(sys.argv[1]).resolve()
assert '--live' in sys.argv, 'Requires --live and a systemd user session'
units, processes, owners = [], [], []

def until(predicate):
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(.05)
    raise AssertionError('Lifetime condition not satisfied within eight seconds')

with tempfile.TemporaryDirectory(prefix='widget-lifetime-') as tmp:
    base = Path(tmp)
    config = base / 'core'
    (config / 'bin').mkdir(parents=True)
    proxy = config / 'bin/wl-mitm'
    proxy.write_text('''#!/usr/bin/python3
import os,pathlib,socket,sys,time
p=pathlib.Path(sys.argv[1]).parent
(p/'proxy.pid').write_text(str(os.getpid()))
s=socket.socket(socket.AF_UNIX);s.bind(str(p/'wayland'))
while True:time.sleep(1)
''')
    proxy.chmod(0o755)
    (config / 'payload.py').write_text('''import os,pathlib,subprocess,sys,time
p=pathlib.Path(sys.argv[1]);child=subprocess.Popen(['/usr/bin/sleep','300'])
(p/'payload.pid').write_text(str(os.getpid()))
(p/'descendant.pid').write_text(str(child.pid))
while True:time.sleep(1)
''')
    (config / 'sandbox-launch').write_text(
        '#!/bin/bash\nexec /usr/bin/python3 "$(dirname "$0")/payload.py" "$2"\n')
    owner_code = '''import pathlib,socket,sys,time
p=pathlib.Path(sys.argv[1]);s=socket.socket(socket.AF_UNIX)
s.bind(str(p/'lifetime'));s.listen(1)
print('ready',flush=True)
while True:time.sleep(1)
'''
    def owner(directory):
        process = subprocess.Popen([sys.executable, '-u', '-c', owner_code, str(directory)],
                                   stdout=subprocess.PIPE, text=True)
        owners.append(process)
        assert process.stdout.readline().strip() == 'ready'
        return process

    def start(directory):
        # Renderer selection validates the same manifest used by real packages.
        (directory/'View.qml').write_text('')
        (directory/'widget.json').write_text(json.dumps({'schemaVersion':2,'coreApi':3,'kind':'desktop-widget','id':'io.example.fixture','name':'Resource fixture','version':'0.0.1','entryPoint':'View.qml','families':['small'],'defaultFamily':'small','defaults':{}}))
        unit = f'omarchy-widget-island-lifetime-{os.getpid()}-{len(units)}.service'
        args = json.loads(subprocess.check_output([str(helper), 'resource-plan', unit], text=True))
        # Only the test's fake supervisor owns these fixtures. Do not depend on
        # or activate the real desktop service; retain all resource boundaries.
        args = [a for a in args if not a.startswith(('--property=BindsTo=', '--property=After='))]
        log = (directory / f'worker-{len(units)}.log').open('w+')
        process = subprocess.Popen(['/usr/bin/systemd-run', *args, str(helper),
                                    'island-worker', str(config), str(directory), str(directory)],
                                   stdout=log, stderr=subprocess.STDOUT, text=True)
        units.append(unit)
        processes.append((process, log))
        return process, log

    def failure(process, log):
        process.wait(timeout=10)
        log.flush(); log.seek(0)
        output = log.read()
        assert process.returncode != 0, output
        return output

    try:
        # Baseline regression: a stale generation must never start package code,
        # even if systemd replays its start after a service restart.
        stale = base / 'stale'; stale.mkdir()
        supervisor = owner(stale); supervisor.kill(); supervisor.wait(timeout=3)
        process, log = start(stale)
        failure(process, log)
        assert not (stale / 'proxy.pid').exists(), 'Stale worker started the proxy'
        assert not (stale / 'payload.pid').exists(), 'Stale worker started package code'
        print('PASS: stale generation refused before proxy/package startup', flush=True)

        live = base / 'live'; live.mkdir()
        supervisor = owner(live)
        process, log = start(live)
        until(lambda: (live / 'descendant.pid').exists())
        pids = [(live / name).read_text() for name in ['proxy.pid', 'payload.pid', 'descendant.pid']]
        assert process.poll() is None
        # SIGKILL prevents any cooperative owner cleanup. Worker must detect the
        # closed lifetime socket, and systemd must clear every descendant.
        started = time.monotonic()
        supervisor.kill(); supervisor.wait(timeout=3)
        failure(process, log)
        until(lambda: all(not Path('/proc', pid).exists() for pid in pids))
        print(f'PASS: supervisor SIGKILL stops worker/proxy/payload/descendant in {time.monotonic()-started:.2f}s', flush=True)
        for name in ['proxy.pid', 'payload.pid', 'descendant.pid']:
            (live / name).unlink()
        process, log = start(live)
        failure(process, log)
        assert not (live / 'proxy.pid').exists(), 'Replay started stale code'
        print('PASS: replay of the dead generation remains refused', flush=True)
    finally:
        for process in owners:
            if process.poll() is None: process.kill()
            process.wait(timeout=3)
        for unit in units:
            subprocess.run(['/usr/bin/systemctl', '--user', 'stop', unit],
                           capture_output=True, timeout=10)
        for process, log in processes:
            if process.poll() is None: process.kill()
            process.wait(timeout=10); log.close()
