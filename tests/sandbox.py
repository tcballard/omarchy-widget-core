#!/usr/bin/env python3
"""Production argv assertions and live mount/namespace denial probes."""
import json, os, shutil, socket, subprocess, sys, tempfile
from pathlib import Path
root=Path(__file__).resolve().parents[1]
source=(root/'sandbox-launch').read_text()
with tempfile.TemporaryDirectory(prefix='widget-island-') as temp:
    base=Path(temp);config=base/'core';config.mkdir();(config/'bin').mkdir()
    for name in ['Host.qml','shell.qml','bin/omarchy-widget']:(config/name).write_text('fixture')
    for name in ['qml','Commons','Ui']:(config/name).mkdir()
    package=base/'package';package.mkdir();(package/'code').write_text('immutable')
    home=base/'home';home.mkdir();(home/'secret').write_text('secret')
    runtime=base/'runtime';runtime.mkdir()
    sockets=[]
    for name in ['broker','filtered','raw','agent']:
        if '--live' in sys.argv:
            sock=socket.socket(socket.AF_UNIX);sock.bind(str(runtime/name));sockets.append(sock)
        else:(runtime/name).touch()
    env={**os.environ,'HOME':str(home),'SSH_AUTH_SOCK':str(runtime/'agent'),'SANDBOX_SECRET':'secret'}
    capture=base/'capture';capture.write_text('#!/usr/bin/python3\nimport sys,json\nprint(json.dumps(sys.argv[1:]))\n');capture.chmod(0o755)
    plan=config/'sandbox-launch'
    plan.write_text(source.replace('/usr/bin/bwrap',str(capture)).replace('-S "$3"','-f "$3"').replace('-S "$4"','-f "$4"') if '--live' not in sys.argv else source.replace('/usr/bin/bwrap',str(capture)))
    command=['bash',str(plan),'runner',str(package),str(runtime/'broker'),str(runtime/'filtered')]
    args=json.loads(subprocess.check_output(command,env=env,text=True))
    assert all(flag in args for flag in ['--unshare-all','--clearenv','--new-session','--die-with-parent'])
    assert '--bind' not in args and '--dev-bind' not in args and '--share-net' not in args
    mounts=[tuple(args[i:i+3]) for i,a in enumerate(args) if a=='--ro-bind']
    assert ('--ro-bind',str(package),'/widget') in mounts
    assert ('--ro-bind',str(runtime/'filtered'),'/run/widget-core/wayland-0') in mounts
    assert ('--ro-bind',str(runtime/'broker'),'/run/widget-core/broker') in mounts
    assert all(m[1] not in [str(home),str(runtime),str(config),'/etc','/proc','/dev'] for m in mounts)
    assert str(runtime/'raw') not in args and str(runtime/'agent') not in args
    assert args[-4:]==['/usr/bin/qs','--no-duplicate','-p','/app']
    print('PASS: no shared registry or writable host mounts; one package; two scoped sockets; clean environment')
    absent=config/'absent';absent.write_text(source.replace('/usr/bin/bwrap',str(base/'missing')))
    assert subprocess.run(['bash',str(absent),'check'],env=env,capture_output=True).returncode
    if '--live' not in sys.argv:sys.exit(0)
    probe=config/'qml/probe.py'
    probe.write_text('''import os,pathlib,socket
assert 'SSH_AUTH_SOCK' not in os.environ and 'SANDBOX_SECRET' not in os.environ
for path in '''+repr([str(home),str(runtime/'raw'),str(runtime/'agent'),'/run/dbus/system_bus_socket','/sys'])+''':
    assert not pathlib.Path(path).exists(), path
try:pathlib.Path('/widget/code').write_text('attack')
except OSError:pass
else:raise AssertionError('Writable package')
assert not pathlib.Path('/proc/'''+str(os.getpid())+'''/environ').exists()
assert pathlib.Path('/run/widget-core/broker').is_socket()
pathlib.Path('/tmp/scratch').write_text('ephemeral')
print('PASS: actual package sandbox denies host files, raw display, other sockets, code writes and host processes')
''')
    plan.write_text(source.replace('/usr/bin/qs --no-duplicate -p /app','/usr/bin/python3 /app/qml/probe.py'))
    subprocess.run(command,env=env,check=True,timeout=20)
