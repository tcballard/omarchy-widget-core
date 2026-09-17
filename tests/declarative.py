#!/usr/bin/env python3
"""Data-only rendering, settings and optional real-registry migration tests.
Qt offscreen is not compositor or memory evidence.
"""
import json, os, subprocess, sys, tempfile
from pathlib import Path
os.environ.setdefault('QT_QPA_PLATFORM','offscreen')
from PySide6.QtCore import QUrl, QMetaObject, QObject, Q_ARG
from PySide6.QtGui import QGuiApplication
from PySide6.QtQuick import QQuickWindow
from PySide6.QtQml import QQmlApplicationEngine
from PySide6.QtTest import QTest
root=Path(__file__).resolve().parents[1]
manifest=json.loads((root/'examples/declarative-clock/widget.json').read_text())
with tempfile.TemporaryDirectory() as directory:
    temp=Path(directory)
    if len(sys.argv)>1:
        binary=str(Path(sys.argv[1]).resolve())
        env={**os.environ,'XDG_DATA_HOME':str(temp/'data'),'XDG_STATE_HOME':str(temp/'state')}
        env.pop('OMARCHY_WIDGET_BROKER',None)
        def cli(*args,ok=True):
            result=subprocess.run([binary,*map(str,args)],env=env,capture_output=True,text=True)
            assert (result.returncode==0)==ok,result.stdout+result.stderr
            return json.loads(result.stdout)
        package=temp/'clock';package.mkdir();(package/'View.qml').write_text('import QtQuick\nItem {}')
        old={k:v for k,v in manifest.items() if k not in ['renderer','requires','settingsSchema','settingsUi','view','settingsVersion','migrations']}
        old['entryPoint']='View.qml';old['version']='0.1.2'
        old['defaults']={'cities':[{'label':'My Tokyo','zone':'Asia/Tokyo'}],'displayMode':'analogue','homeZone':'Asia/Tokyo'}
        (package/'widget.json').write_text(json.dumps(old))
        cli('install',package);first=cli('create',manifest['id'],'small')['updated']
        second=cli('create',manifest['id'],'large')['updated']
        before=cli('list')['installed']
        cli('update',root/'examples/declarative-clock')
        after=cli('list')['installed']
        assert len(after)==2 and all(e['manifest']['renderer']=='declarative' for e in after)
        assert [e['placement']['settings'] for e in after]==[e['placement']['settings'] for e in before]
        exported=cli('export-settings',manifest['id']);assert exported
        bad=json.loads(json.dumps(manifest));bad['entryPoint']='View.qml'
        (package/'widget.json').write_text(json.dumps(bad));cli('validate',package,ok=False)
        # Clock service uses one bounded Rust invocation, with invalid paths ignored.
        times=cli('clock-times',json.dumps(['UTC','Asia/Tokyo','../../etc/passwd']))
        assert set(times['zones'])=={'UTC','Asia/Tokyo'}
        # Rollback returns both instances to the original renderer/settings.
        cli('rollback',manifest['id']);assert all('renderer' not in e['manifest'] for e in cli('list')['installed'])
        cli('update',root/'examples/declarative-clock')
        e=next(e for e in cli('list')['installed'] if e['instanceId']==first)
        revised={**e['placement']['settings'],'displayMode':'digital'}
        cli('save',first,json.dumps({'revision':e['placement']['revision'],'settings':revised}))
        cli('save',first,json.dumps({'revision':e['placement']['revision'],'settings':revised}),ok=False)
        invalid={**revised,'cities':[{'label':'Bad','zone':'../../etc/passwd'}]}
        cli('configure',first,json.dumps(invalid),ok=False)
        sibling=next(e for e in cli('list')['installed'] if e['instanceId']==second)
        assert sibling['placement']['settings']['displayMode']=='analogue'
    for name in ['Commons','Ui']:
        (temp/'qs').mkdir(exist_ok=True);(temp/'qs'/name).symlink_to(root/name,target_is_directory=True)
    fixture=temp/'Test.qml'
    fixture.write_text('''import QtQuick
import qs.Commons
import "CORE" as Core
Window {
    id:harness
    width:394;height:394;visible:true
    property var definition:DEFINITION
    QtObject {id:context;property var settings:definition.defaults;property string family:"large"}
    QtObject {id:editorContext;property var draftSettings:JSON.parse(JSON.stringify(definition.defaults))}
    Core.DeclarativeView {id:view;anchors.fill:parent;widgetContext:context;definition:harness.definition;clockTimes:({"Europe/London":{time:"12:34",hour:12,minute:34}})}
    Core.DeclarativeSettings {id:editor;objectName:"editor";anchors.fill:parent;visible:false;settingsContext:editorContext;definition:harness.definition}
    function edit() {view.visible=false;editor.visible=true;}
    function amend() {editor.change("displayMode","analogue");editor.city("cities",0,"label","Changed");}
    function verify() {return editorContext.draftSettings.displayMode==="analogue" && editorContext.draftSettings.cities[0].label==="Changed" && context.settings.cities[0].label==="London";}
    function analogue() {editor.visible=false;view.visible=true;context.settings=Object.assign({},context.settings,{displayMode:"analogue"});}
}
'''.replace('CORE',(root/'qml').as_uri()).replace('DEFINITION',json.dumps(manifest)))
    app=QGuiApplication([]);engine=QQmlApplicationEngine();engine.addImportPath(str(temp));warnings=[]
    engine.warnings.connect(lambda es:warnings.extend(e.toString() for e in es));engine.load(QUrl.fromLocalFile(str(fixture)))
    assert engine.rootObjects(),warnings
    window=engine.rootObjects()[0];QTest.qWait(150)
    assert not window.grabWindow().isNull()
    QMetaObject.invokeMethod(window,'edit');QTest.qWait(50)
    QMetaObject.invokeMethod(window,'amend');QTest.qWait(50)
    from PySide6.QtCore import Q_RETURN_ARG
    assert QMetaObject.invokeMethod(window,'verify',Q_RETURN_ARG('QVariant'))
    QMetaObject.invokeMethod(window,'analogue');QTest.qWait(100)
    out=root/'test-results/declarative';out.mkdir(parents=True,exist_ok=True)
    assert window.grabWindow().save(str(out/'analogue.png'))
    assert not warnings,'\n'.join(warnings)
print('PASS: declarative view, generated settings draft isolation'+('; registry update/rollback, independent instances, clock service and schema rejection' if len(sys.argv)>1 else ' (registry tests not requested)'))
