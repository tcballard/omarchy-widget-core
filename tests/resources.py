#!/usr/bin/env python3
"""Real systemd user/cgroup-v2 tests with production worker and policy.
Use only with a disposable user manager; refuses to replace an existing Core unit.
The display proxy and widget workload are explicit fixtures, not a graphical test.
"""
import json, os, shutil, socket, subprocess, sys, tempfile, time
from pathlib import Path
helper=Path(sys.argv[1]).resolve()
assert '--live' in sys.argv, 'Requires explicit --live in a disposable user session'
host='omarchy-widget-host.service'
def ctl(*args,check=True):return subprocess.run(['/usr/bin/systemctl','--user',*args],text=True,capture_output=True,check=check,timeout=15)
def property(unit,key):return ctl('show',unit,'--property='+key,'--value',check=False).stdout.strip()
assert property(host,'LoadState') in ('not-found',''), 'An existing Core unit must not be replaced by this test'
def until(test,seconds=20):
    end=time.monotonic()+seconds
    while time.monotonic()<end:
        result=test()
        if result:return result
        time.sleep(.1)
    raise AssertionError('Timed out waiting for test condition')
workers=[];units=[];host_created=False
with tempfile.TemporaryDirectory(prefix='widget resources $literal ') as temp:
    base=Path(temp);config=base/'core';(config/'bin').mkdir(parents=True)
    shutil.copy2(helper,config/'bin/omarchy-widget')
    proxy=config/'bin/wl-mitm'
    proxy.write_text('''#!/usr/bin/python3
import os,pathlib,socket,sys,time
p=pathlib.Path(sys.argv[1]).parent
(p/'proxy.pid').write_text(str(os.getpid()))
s=socket.socket(socket.AF_UNIX);s.bind(str(p/'wayland'))
while True:time.sleep(.1)
''');proxy.chmod(0o755)
    payload=config/'payload.py'
    payload.write_text('''import os,pathlib,subprocess,sys,time
source=pathlib.Path(sys.argv[1]); role=(source/'role').read_text(); out=source
(out/'payload.pid').write_text(str(os.getpid()))
if role=='healthy':
    while True:
        (out/'heartbeat').write_text(str(time.monotonic()))
        time.sleep(.1)
elif role=='memory':
    (out/'ready').touch()
    while not (out/'go').exists():time.sleep(.05)
    allocations=[]
    while True:allocations.append(bytearray(8*1024*1024))
elif role=='cpu':
    while True:pass
elif role=='tasks':
    children=[]
    try:
        for i in range(100):children.append(subprocess.Popen(['/usr/bin/sleep','60']))
    except OSError:
        (out/'tasks-denied').write_text(str(len(children)))
    else:raise AssertionError('Task limit was not enforced')
    while True:time.sleep(.1)
''')
    (config/'sandbox-launch').write_text('#!/bin/bash\nconfig=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)\nexec /usr/bin/python3 "$config/payload.py" "$2"\n')
    def start(role):
        source=base/(role+'-source');source.mkdir();(source/'role').write_text(role)
        directory=base/role;directory.mkdir();(directory/'wayland.toml').write_text('fixture')
        unit=f'omarchy-widget-island-test-{role}-{os.getpid()}.service'
        args=json.loads(subprocess.check_output([str(helper),'resource-plan',unit],text=True))
        process=subprocess.Popen(['/usr/bin/systemd-run',*args,str(helper),'island-worker',str(config),str(source),str(directory)],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
        workers.append(process);units.append(unit)
        until(lambda:(source/'payload.pid').exists())
        group=Path('/sys/fs/cgroup')/property(unit,'ControlGroup').lstrip('/')
        assert (group/'memory.max').read_text().strip()=='268435456'
        assert (group/'memory.swap.max').read_text().strip()=='0'
        assert (group/'pids.max').read_text().strip()=='64'
        quota,period=map(int,(group/'cpu.max').read_text().split());assert quota*4<=period
        assert (group/'memory.oom.group').read_text().strip()=='1'
        return unit,process,source,directory,group
    try:
        # Unsupported/unlimited cgroups must fail before launching either fixture.
        denied=subprocess.run([str(helper),'island-worker',str(config),str(base),str(base)],capture_output=True,text=True,timeout=5)
        assert denied.returncode and 'resource preflight failed' in denied.stdout
        subprocess.run(['/usr/bin/systemd-run','--user','--unit='+host,'--property=MemoryMax=512M','--property=TasksMax=256','--property=CPUQuota=100%','/usr/bin/sleep','300'],check=True,timeout=15)
        host_created=True
        healthy=start('healthy'); hsource=healthy[2]; until(lambda:(hsource/'heartbeat').exists())
        hostgroup=Path('/sys/fs/cgroup')/property(host,'ControlGroup').lstrip('/')
        assert healthy[4].parent==hostgroup.parent and healthy[4]!=hostgroup
        memory=start('memory');until(lambda:(memory[2]/'ready').exists())
        heartbeat=(hsource/'heartbeat').read_text();(memory[2]/'go').touch()
        until(lambda:memory[1].poll() is not None)
        assert memory[1].returncode!=0
        until(lambda:float((hsource/'heartbeat').read_text() or 0)>float(heartbeat))
        assert property(host,'ActiveState')=='active' and property(healthy[0],'ActiveState')=='active'
        for pidfile in [memory[2]/'payload.pid',memory[3]/'proxy.pid']:
            until(lambda:not Path('/proc',pidfile.read_text()).exists())
        print('PASS: memory exhaustion kills only its package, including its proxy; sibling and Core survive')
        cpu=start('cpu')
        def throttled():
            stats=dict(line.split() for line in (cpu[4]/'cpu.stat').read_text().splitlines())
            return int(stats['nr_throttled'])>0
        until(throttled)
        assert property(healthy[0],'ActiveState')=='active';ctl('stop',cpu[0]);cpu[1].wait(timeout=10)
        tasks=start('tasks');until(lambda:(tasks[2]/'tasks-denied').exists())
        assert 0<int((tasks[2]/'tasks-denied').read_text())<64
        assert property(host,'ActiveState')=='active' and property(healthy[0],'ActiveState')=='active'
        print('PASS: actual CPU throttling and task creation denial leave Core and sibling active')
        ctl('stop',host)
        for process in workers:process.wait(timeout=10)
        for unit in units:assert property(unit,'ActiveState') not in ('active','activating','deactivating')
        print('PASS: stopping Core stops all package services and descendants; literal dollar/space paths preserved')
    finally:
        for unit in units:ctl('stop',unit,check=False)
        if host_created:ctl('stop',host,check=False);ctl('reset-failed',host,check=False)
        for process in workers:
            if process.poll() is None:process.kill()
            process.communicate(timeout=5)
