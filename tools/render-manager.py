#!/usr/bin/env python3
"""Render Core manager fixtures offscreen; these are not live desktop captures."""
import os,json,tempfile,sys
from pathlib import Path
os.environ['QT_QPA_PLATFORM']='offscreen'
os.environ['QT_QPA_PLATFORMTHEME']='basic'
os.environ['QT_QUICK_BACKEND']='software'
from PySide6.QtCore import QUrl,QMetaObject,Q_ARG,Q_RETURN_ARG,Qt,QPoint
from PySide6.QtGui import QGuiApplication
from PySide6.QtQml import QQmlApplicationEngine
from PySide6.QtQuick import QQuickWindow
from PySide6.QtTest import QTest
root=Path(__file__).resolve().parents[1]
out=root/'test-results/manager-design';out.mkdir(exist_ok=True)
app=QGuiApplication([])
manifest=json.loads((root/'examples/declarative-clock/widget.json').read_text())
package={'packageId':manifest['id'],'manifest':manifest,'directory':str(root/'examples/declarative-clock'),'instanceCount':2,'health':{'state':'running','failures':0}}
entries=[]
for i in range(2):
 entries.append(dict(package,instanceId=f'clock-{i}',placement={'size':['large','small'][i],'enabled':i==0,'monitor':'eDP-1','workspace':None if i==0 else 2,'cell':{'column':i,'row':0}},effective={'monitor':'eDP-1','column':i,'row':0}))
with tempfile.TemporaryDirectory() as d:
 t=Path(d);(t/'qs').mkdir();(t/'qs/Commons').symlink_to(root/'Commons',target_is_directory=True)
 qml='''import QtQuick
import qs.Commons
import "CORE" as Core
Window {
 id: win; width:760; height:660; visible:true
 Core.Manager { id:manager;anchors.fill:parent;entries:ENTRIES;catalog:CATALOG }
 function mode(view,width,light) {
   win.width=width; manager.view=view;
   Color.apply(light ? {foreground:"#1e293b",background:"#f5f6f8",accent:"#176ab6",red:"#b3261e"} : {});
   return true;
 }
 function state(value) {
   manager.instanceDetails=({}); manager.packageDetails=({}); manager.confirmation=null;
   manager.entries=ENTRIES; manager.catalog=CATALOG;
   if(value==="empty")manager.entries=[];
   if(value==="details")manager.toggleDetails("instance","clock-0");
   if(value==="package")manager.toggleDetails("package",ID);
   if(value==="confirm")manager.ask("instance","clock-0","World Clock");
   if(value==="error")manager.error="Couldn't save the change. Your previous settings are safe. Try again.";
   else manager.error="";
   return true;
 }
}
'''.replace('CORE',(root/'qml').as_uri()).replace('ENTRIES',json.dumps(entries)).replace('CATALOG',json.dumps([package])).replace('ID',json.dumps(manifest['id']))
 (t/'Preview.qml').write_text(qml)
 engine=QQmlApplicationEngine();engine.addImportPath(str(t));warnings=[];engine.warnings.connect(lambda es:warnings.extend(e.toString() for e in es));engine.load(QUrl.fromLocalFile(str(t/'Preview.qml')));assert engine.rootObjects(),warnings
 win=engine.rootObjects()[0]
 def call(name,*args):return QMetaObject.invokeMethod(win,name,Q_RETURN_ARG('QVariant'),*[Q_ARG('QVariant',a) for a in args])
 for light in [True,False]:
  for view,detail,width in [('instances','normal',760),('available','normal',760),('instances','details',760),('available','package',760),('instances','empty',760),('instances','confirm',760),('instances','error',480),('available','normal',480)]:
   call('mode',view,width,light);call('state',detail);QTest.qWait(150)
   assert win.grabWindow().save(str(out/f'{"light" if light else "dark"}-{view}-{detail}-{width}.png'))
 assert not warnings,'\n'.join(warnings)
 print('PASS: 16 actual QML renders; light/dark, instances/gallery, details, confirmation, empty/error, 480/760 widths; no QML warnings')
