#!/usr/bin/env python3
"""Policy argv checks, plus real Bubblewrap denial probes when --live is given.
Only the check payload is replaced; production mount/namespace arguments are used.
A denied/unavailable namespace is a FAIL, never a passing skip.
"""
import json, os, shutil, socket, subprocess, sys, tempfile
from pathlib import Path
root=Path(__file__).resolve().parents[1]
source=(root/'sandbox-launch').read_text()
with tempfile.TemporaryDirectory(prefix='widget-sandbox-') as directory:
    base=Path(directory); config=base/'core';config.mkdir()
    home=base/'home';home.mkdir()
    data=home/'.local/share/omarchy/widgets';data.mkdir(parents=True)
    state=home/'.local/state/omarchy/widgets';state.mkdir(parents=True)
    (home/'secret').write_text('must not be visible')
    (data/'code').write_text('read only')
    (config/'code').write_text('read only')
    runtime=base/'runtime';runtime.mkdir()
    if '--live' in sys.argv:
        server=socket.socket(socket.AF_UNIX);server.bind(str(runtime/'wayland-test'))
        agent=socket.socket(socket.AF_UNIX);agent.bind(str(runtime/'agent-secret'))
    else:
        (runtime/'wayland-test').touch();(runtime/'agent-secret').touch()
    env={**os.environ,'HOME':str(home),'XDG_DATA_HOME':str(home/'.local/share'),'XDG_STATE_HOME':str(home/'.local/state'),'XDG_RUNTIME_DIR':str(runtime),'WAYLAND_DISPLAY':'wayland-test','SSH_AUTH_SOCK':str(runtime/'agent-secret'),'SANDBOX_SECRET':'must-not-inherit'}
    # Capture the production argument construction without requiring namespaces.
    capture=base/'capture'; capture.write_text('#!/usr/bin/python3\nimport json,sys\nprint(json.dumps(sys.argv[1:]))\n');capture.chmod(0o755)
    plan=config/'sandbox-launch';plan.write_text(source.replace('/usr/bin/bwrap',str(capture)).replace('[[ -S', '[[ -f'))
    args=json.loads(subprocess.check_output(['bash',str(plan),'run'],env=env,text=True))
    for flag in ['--unshare-all','--clearenv','--new-session','--die-with-parent']:
        assert flag in args
    assert '--share-net' not in args and '--dev-bind' not in args
    mounts=[]; i=0
    while i<len(args):
        if args[i] in ['--bind','--ro-bind']:
            mounts.append(tuple(args[i:i+3]));i+=3
        else:i+=1
    assert [m for m in mounts if m[0]=='--bind']==[('--bind',str(state),str(state))]
    assert ('--ro-bind',str(runtime/'wayland-test'),'/run/widget-core/wayland-0') in mounts
    assert all(m[1] not in [str(home),str(runtime),'/etc','/proc','/sys','/dev'] for m in mounts)
    assert not any('agent-secret' in arg or 'must-not-inherit' in arg for arg in args)
    assert args[-4:]==['/usr/bin/qs','--no-duplicate','-p',str(config)]
    # Missing Bubblewrap must not reach an unsandboxed qs invocation.
    absent=config/'absent';absent.write_text(source.replace('/usr/bin/bwrap',str(base/'missing-bwrap')))
    assert subprocess.run(['bash',str(absent),'check'],env=env,capture_output=True).returncode!=0
    print('PASS: narrow read-only mounts; only Core state writable; private namespaces/environment; no unsandboxed fallback')
    if '--live' not in sys.argv:sys.exit(0)
    tcp=socket.socket();tcp.bind(('127.0.0.1',0));tcp.listen()
    probe=config/'probe.py'
    probe.write_text('''import os,pathlib,socket
home=pathlib.Path(os.environ['HOME'])
assert not (home/'secret').exists()
assert 'SSH_AUTH_SOCK' not in os.environ and 'SANDBOX_SECRET' not in os.environ
assert not pathlib.Path('/run/dbus/system_bus_socket').exists()
assert not pathlib.Path('/sys').exists()
assert not pathlib.Path('''+repr(str(runtime/'agent-secret'))+''').exists()
for path in ['''+repr(str(data/'code'))+','+repr(str(config/'code'))+''']:
    try:pathlib.Path(path).write_text('forbidden')
    except OSError:pass
    else:raise AssertionError('Read-only mount was writable: '+path)
# The host process and its credentials cannot be inspected through /proc.
assert not pathlib.Path('/proc/'''+str(os.getpid())+'''/environ').exists()
s=socket.socket();s.settimeout(1)
try:s.connect(('127.0.0.1','''+str(tcp.getsockname()[1])+'''))
except OSError:pass
else:raise AssertionError('Host network is reachable')
pathlib.Path('''+repr(str(state/'probe-result'))+''').write_text('state persists')
print('PASS: real sandbox denies home/agents, code writes, host processes and host network; state persists')
''')
    plan.write_text(source.replace('-- /usr/bin/true','-- /usr/bin/python3 "$config/probe.py"'))
    result=subprocess.run(['bash',str(plan),'check'],env=env,text=True,capture_output=True,timeout=20)
    print(result.stdout,end='')
    if result.returncode:
        print(result.stderr,file=sys.stderr);raise SystemExit(result.returncode)
    assert (state/'probe-result').read_text()=='state persists'
