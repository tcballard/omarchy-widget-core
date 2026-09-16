#!/usr/bin/env python3
"""Installer transaction tests with explicit command fixtures, no live service claim."""
import json, os, shutil, subprocess, tempfile
from pathlib import Path
root=Path(__file__).resolve().parents[1]
stub=r'''#!/usr/bin/env python3
import json,os,pathlib,sys
name=pathlib.Path(sys.argv[0]).name
args=sys.argv[1:]
if name=='cargo':
    manifest=pathlib.Path(args[args.index('--manifest-path')+1])
    helper=manifest.parent/'target/release/omarchy-widget'
    helper.parent.mkdir(parents=True,exist_ok=True)
    helper.write_text("#!/usr/bin/env python3\nimport os,sys\nif len(sys.argv)>1 and sys.argv[1]=='resource-preflight' and os.getenv('FAIL_RESOURCE'):sys.exit(1)\nprint('{}')\n")
    helper.chmod(0o755)
elif name=='omarchy':
    if args[:2]==['plugin','list']:print(json.dumps([{'id':'io.github.tcballard.widget-core'}]))
    if args[:2]==['plugin','enable'] and os.getenv('FAIL_ENABLE'):sys.exit(1)
elif name=='systemctl':
    if 'start' in args and os.getenv('FAIL_START'):sys.exit(1)
'''
for failure in ['', 'FAIL_ENABLE', 'FAIL_START', 'FAIL_SANDBOX', 'FAIL_RESOURCE']:
    with tempfile.TemporaryDirectory(prefix='widget install ') as tmp:
        base=Path(tmp); home=base/'home with spaces';home.mkdir()
        repo=base/'source with spaces';shutil.copytree(root,repo,ignore=shutil.ignore_patterns('target','test-results'))
        (repo/'build-wayland-filter').write_text('#!/usr/bin/bash\nmkdir -p target/wl-mitm-source/target/release\nprintf fake > target/wl-mitm-source/target/release/wl-mitm\n')
        (repo/'sandbox-launch').write_text('#!/usr/bin/bash\n[[ -z ${FAIL_SANDBOX:-} ]]\n')
        commands=base/'commands';commands.mkdir()
        for name in ['cargo','omarchy','omarchy-shell','systemctl','qs','bwrap']:
            file=commands/name;file.write_text(stub);file.chmod(0o755)
        destination=home/'.config/omarchy/plugins/io.github.tcballard.widget-core'
        destination.mkdir(parents=True);(destination/'previous').write_text('old core')
        unit=home/'.config/systemd/user/omarchy-widget-host.service';unit.parent.mkdir(parents=True);unit.write_text('old unit')
        launcher=home/'.local/bin/omarchy-widget';launcher.parent.mkdir(parents=True);launcher.write_text('old launcher')
        desktop=home/'.local/share/applications/io.github.tcballard.widget-core.desktop'
        desktop.parent.mkdir(parents=True);desktop.write_text('old desktop entry')
        env={**os.environ,'HOME':str(home),'PATH':str(commands)+':'+os.environ['PATH']}
        env.pop('XDG_DATA_HOME',None)
        if failure:env[failure]='1'
        result=subprocess.run(['bash',str(repo/'install-local'),'--update'],env=env,text=True,capture_output=True,cwd=repo)
        if failure:
            assert result.returncode!=0,result.stdout
            assert (destination/'previous').read_text()=='old core'
            assert unit.read_text()=='old unit'
            assert launcher.read_text()=='old launcher'
            assert desktop.read_text()=='old desktop entry'
        else:
            assert result.returncode==0,result.stderr
            assert (destination/'Host.qml').is_file()
            assert 'Name=Widgets\n' in desktop.read_text()
            assert 'Exec=omarchy-widget manage\n' in desktop.read_text()
            assert (destination/'Commons/Color.qml').is_file()
            assert 'KillMode=control-group' in unit.read_text()
            assert 'No widget code' in (destination/'Service.qml').read_text()
print('PASS: installer paths with spaces; complete host payload; activation/start failure restores Core, service and launcher')
