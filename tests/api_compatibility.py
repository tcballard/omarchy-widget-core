#!/usr/bin/env python3
"""The deliberately older host must reject API 3 before touching registry state."""
import json,os,subprocess,sys,tempfile
from pathlib import Path
root=Path(__file__).resolve().parents[1];current=Path(sys.argv[1]).resolve();older=Path(sys.argv[2]).resolve()
with tempfile.TemporaryDirectory() as tmp:
    base=Path(tmp);env={**os.environ,'XDG_DATA_HOME':str(base/'data'),'XDG_STATE_HOME':str(base/'state')}
    def run(binary,*args):return subprocess.run([str(binary),*map(str,args)],env=env,text=True,capture_output=True)
    for source in [root/'sdk/starter',root/'examples/countdown',root/'examples/weather']:
        assert run(current,'validate',source).returncode==0
        before=run(older,'list').stdout
        for operation in ['validate','install']:
            result=run(older,operation,source);assert result.returncode!=0,(source,operation,result.stdout)
            assert 'schema/coreApi' in result.stdout,result.stdout
        assert run(older,'list').stdout==before
        assert not (base/'state/omarchy/widgets/layout.json').exists()
print('PASS: API 3 schema/action/weather packages accepted by current Core and rejected by the historical host without registry mutation')
