#!/usr/bin/env python3
"""External-author workflow against the real binary; no compositor claim."""
import json,os,subprocess,sys,tempfile
from pathlib import Path
binary=Path(sys.argv[1]).resolve();root=Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory() as tmp:
    base=Path(tmp);package=base/'A package';env={**os.environ,'XDG_DATA_HOME':str(base/'data'),'XDG_STATE_HOME':str(base/'state')}
    def call(*args,ok=True):
        p=subprocess.run([str(binary),*map(str,args)],env=env,text=True,capture_output=True)
        assert (p.returncode==0)==ok,p.stdout+p.stderr
        return json.loads(p.stdout)
    id='io.example.author-workflow'
    call('new',package,id,'Author example');original=(package/'View.qml').read_text()
    call('new',package,id,'Do not overwrite',ok=False);assert (package/'View.qml').read_text()==original
    call('validate',package)
    subprocess.run([sys.executable,str(root/'sdk/render_previews.py'),str(package/'previews')],check=True)
    m=json.loads((package/'widget.json').read_text());m['previews']={k:'previews/'+k+'.png' for k in ['small','medium','large']};(package/'widget.json').write_text(json.dumps(m))
    call('validate',package);call('install',package)
    a=call('create',id,'small')['updated'];b=call('create',id,'large')['updated'];assert a!=b
    def entry(instance):return next(e for e in call('list')['installed'] if e['instanceId']==instance)
    for instance,title in [(a,'London'),(b,'Tokyo')]:
        e=entry(instance);call('save',instance,json.dumps({'revision':e['placement']['revision'],'settings':{'title':title,'note':'Independent'}}))
    call('save',a,json.dumps({'revision':entry(a)['placement']['revision'],'settings':{'title':12}}),ok=False)
    assert entry(a)['placement']['settings']['title']=='London';assert entry(b)['placement']['settings']['title']=='Tokyo'
    call('hide',a);assert entry(b)['placement']['enabled']
    call('remove-instance',a);assert entry(b)['placement']['settings']['title']=='Tokyo'
    call('uninstall',id,'keep');assert call('list')['retained'][0]['instanceId']==b
    call('install',package);assert not entry(b)['placement']['enabled'];assert entry(b)['placement']['settings']['title']=='Tokyo'
    # New defaults never replace an existing instance's independent settings.
    m['version']='0.2.0';m['defaults']['title']='Changed default';(package/'widget.json').write_text(json.dumps(m));call('update',package)
    assert entry(b)['placement']['settings']['title']=='Tokyo';call('rollback',id)
    assert entry(b)['placement']['settings']['title']=='Tokyo'
print('PASS: scaffold, no overwrite, previews, validation, install, two independent instances, hide/remove, retained reinstall, update and rollback')
