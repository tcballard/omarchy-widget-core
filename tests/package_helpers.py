#!/usr/bin/env python3
"""Install/update/rollback a compiled helper; optional actual sandbox execution."""
import json,os,shutil,subprocess,sys,tempfile
from pathlib import Path
binary=Path(sys.argv[1]).resolve()
with tempfile.TemporaryDirectory() as tmp:
    base=Path(tmp);env={**os.environ,'XDG_DATA_HOME':str(base/'data'),'XDG_STATE_HOME':str(base/'state')}
    def cli(*args):return json.loads(subprocess.check_output([str(binary),*map(str,args)],env=env,text=True))
    package=base/'helper-package';cli('new',package,'io.example.helper','Helper')
    shutil.copy('/usr/bin/true',package/'helper');(package/'helper').chmod(0o6755)
    for operation in ['install','update','rollback']:
        cli(operation,'io.example.helper' if operation=='rollback' else package)
        installed=Path(cli('list')['catalog'][0]['directory'])
        assert (installed/'helper').stat().st_mode & 0o7777 == 0o700
        assert (installed/'View.qml').stat().st_mode & 0o7777 == 0o600
        subprocess.run([str(installed/'helper')],check=True)
print('PASS: compiled helper executes after install, update and rollback; data stays non-executable; privileged mode bits stripped')
