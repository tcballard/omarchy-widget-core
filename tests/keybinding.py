"""Personal binding transaction with explicit Hyprland fixtures."""
import os,subprocess,tempfile
from pathlib import Path
root=Path(__file__).resolve().parents[1]
for syntax in ['lua','conf']:
 for mode in ['success','conflict','reload-failure','inactive']:
  with tempfile.TemporaryDirectory() as tmp:
   home=Path(tmp); config=home/'config with spaces';hypr=config/'hypr';hypr.mkdir(parents=True)
   (hypr/('hyprland.'+syntax)).write_text('fixture')
   target=hypr/('bindings.'+syntax);original='-- user setting\n' if syntax=='lua' else '# user setting\n';target.write_text(original)
   commands=home/'bin';commands.mkdir();ctl=commands/'hyprctl'
   ctl.write_text('''#!/usr/bin/env python3
import os,sys,json
from pathlib import Path
mode=os.environ['MODE'];target=Path(os.environ['BINDINGS'])
if sys.argv[1]=='reload':sys.exit(1 if mode=='reload-failure' else 0)
if mode=='conflict':print(json.dumps([{'modmask':69,'key':'W','keycode':0,'dispatcher':'exec','arg':'another-app'}]))
elif mode!='inactive' and 'omarchy-widget manage' in target.read_text():print(json.dumps([{'modmask':69,'key':'W','keycode':0,'dispatcher':'exec','arg':'omarchy-widget manage'}]))
else:print('[]')
''');ctl.chmod(0o755)
   env=dict(os.environ,HOME=str(home),XDG_CONFIG_HOME=str(config),MODE=mode,BINDINGS=str(target),PATH=str(commands)+':'+os.environ['PATH'])
   run=lambda:subprocess.run(['bash',str(root/'bind-key')],env=env,capture_output=True,text=True)
   result=run()
   if mode=='success':
    assert result.returncode==0,result.stderr
    saved=target.read_text();assert saved.startswith(original) and saved.count('omarchy-widget manage')==1
    assert run().returncode==0 and target.read_text()==saved
    assert len(list(hypr.glob('bindings.*.widget-backup.*')))==1
   else:
    assert result.returncode!=0
    assert target.read_text()==original
   assert not (hypr/('bindings.'+syntax+'.widget-lock')).exists()
print('PASS: Lua/conf shortcut install, idempotence, conflict refusal, reload/inactive rollback, paths with spaces')
