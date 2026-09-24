#!/usr/bin/env python3
"""API 3 action widget -> actual Host context/queue -> Rust -> reopened state.
Offscreen Qt substitutes windows and transport only; no desktop acceptance claim.
"""
import json, os, shutil, subprocess, sys, tempfile, time
from pathlib import Path
os.environ.setdefault('QT_QPA_PLATFORM','offscreen')
from PySide6.QtCore import QObject,QUrl,QMetaObject,Q_ARG,Q_RETURN_ARG,Qt
from PySide6.QtGui import QGuiApplication
from PySide6.QtQml import QQmlApplicationEngine,qmlRegisterType
from PySide6.QtTest import QTest
from qt_process import Process
root=Path(__file__).resolve().parents[1];binary=Path(sys.argv[1]).resolve()
qmlRegisterType(Process,'Quickshell.Io',1,0,'Process');app=QGuiApplication([])
def call(obj,name,*args):return QMetaObject.invokeMethod(obj,name,Q_RETURN_ARG('QVariant'),*[Q_ARG('QVariant',v) for v in args])
def spin(predicate):
    end=time.monotonic()+10
    while time.monotonic()<end:
        app.processEvents()
        if predicate():return
        QTest.qWait(10)
    raise AssertionError('Timed out')
with tempfile.TemporaryDirectory() as tmp:
    temp=Path(tmp);os.environ.update(XDG_DATA_HOME=str(temp/'data'),XDG_STATE_HOME=str(temp/'state'));os.environ.pop('OMARCHY_WIDGET_BROKER',None)
    def cli(*args):return json.loads(subprocess.check_output([str(binary),*map(str,args)],text=True))
    package=temp/'countdown';shutil.copytree(root/'examples/countdown',package)
    cli('install',package);id='io.example.countdown'
    first=cli('create',id,'small')['updated'];second=cli('create',id,'large')['updated']
    def placement(instance):return next(e['placement'] for e in cli('list')['installed'] if e['instanceId']==instance)
    second_before=placement(second)
    for name in ['Commons','Ui']:shutil.copytree(root/name,temp/'qs'/name)
    io=temp/'Quickshell/Io';io.mkdir(parents=True);(io/'qmldir').write_text('module Quickshell.Io\nStdioCollector 1.0 StdioCollector.qml\n');(io/'StdioCollector.qml').write_text('import QtQuick\nQtObject {property string text:"";property bool waitForEnd:false}')
    source=(root/'Host.qml').read_text()
    controller=source[source.index('Item {'):source.index('    function screenFor(')]
    controller=controller.replace('id: root','id: root;objectName:"controller";anchors.fill:parent',1).replace('Quickshell.env("OMARCHY_WIDGET_ROLE") === "manager"','false').replace('Quickshell.env("OMARCHY_WIDGET_REVEAL") === "1"','false')
    line=next(l for l in controller.splitlines() if 'readonly property string helper:' in l)
    controller=controller.replace(line,'property string helper:'+json.dumps(str(binary)))
    controller=controller.replace('property var installed: []','property var installed:'+json.dumps(cli('list')['installed']))
    context=source[source.index('            Core.Lifecycle {'):source.index('            Core.WidgetFrame {')]
    settings=source[source.index('    QtObject {\n        id: settingsContext'):source.index('    FloatingWindow {\n        title: "Widget settings"')]
    panel=source[source.index('        Core.SettingsPanel {'):source.index('    Variants {')].rsplit('\n    }',1)[0].replace('objectName: "settings-panel"','visible:root.configuring!=="";objectName: "settings-panel"')
    harness='''import QtQuick
import qs.Commons
import "CORE" as Core
Window { width:394;height:394;visible:true
CONTROLLER
SETTINGS
Item {
 id:window;anchors.fill:parent;visible:root.configuring===""
 property string modelData:INSTANCE
 property var config:root.entry(modelData).placement
 property var metadata:root.entry(modelData).manifest
 property string sizeName:config.size
CONTEXT
 Core.WidgetFrame {anchors.fill:parent;title:"Countdown";configurable:true;onConfigureRequested:root.configure(window.modelData)
  Loader {id:display;objectName:"display";anchors.fill:parent;Component.onCompleted:setSource(VIEW,{widgetContext:context})}
 }
}
PANEL
function action(name) { return display.item[name](); }
function pending() { return operation.busy; }
function current() { return configuring; }
function draft() { return JSON.stringify(settingsContext.draftSettings); }
}
}
'''.replace('CORE',(root/'qml').as_uri()).replace('CONTROLLER',controller).replace('SETTINGS',settings).replace('INSTANCE',json.dumps(first)).replace('CONTEXT',context).replace('PANEL',panel).replace('VIEW',json.dumps((package/'View.qml').as_uri()))
    file=temp/'Test.qml';file.write_text('import "'+(root/'qml/Declarative.js').as_uri()+'" as Declarative\n'+harness);engine=QQmlApplicationEngine();engine.addImportPath(str(temp));warnings=[];engine.warnings.connect(lambda e:warnings.extend(x.toString() for x in e));engine.load(QUrl.fromLocalFile(str(file)));assert engine.rootObjects(),warnings
    window=engine.rootObjects()[0];c=window.findChild(QObject,'controller');spin(lambda:not call(c,'pending'))
    assert call(c,'action','start');spin(lambda:not call(c,'pending'));started=placement(first);assert started['settings']['endsAt']>0;assert placement(second)==second_before
    assert call(c,'action','pause');spin(lambda:not call(c,'pending'));paused=placement(first);assert paused['settings']['endsAt']==0
    assert call(c,'action','resume');spin(lambda:not call(c,'pending'));resumed=placement(first);assert resumed['settings']['endsAt']>0
    gear=window.findChild(QObject,'settings-gear');assert gear;frame=window.findChild(QObject,'widget-frame');frame.forceActiveFocus()
    for _ in range(20):
        QTest.keyClick(window,Qt.Key.Key_Tab)
        if gear.property('activeFocus'):break
    assert gear.property('activeFocus'),'Gear cannot be reached by Tab'
    QTest.keyClick(window,Qt.Key.Key_Return);spin(lambda:call(c,'current')==first);draft=call(c,'draft')
    QMetaObject.invokeMethod(gear,'clicked');spin(lambda:not call(c,'pending'));assert call(c,'draft')==draft
    cancel=window.findChild(QObject,'cancel-button');QMetaObject.invokeMethod(cancel,'clicked');spin(lambda:not call(c,'pending'))
    cli('hide',first);cli('add',first);cli('update',package);cli('rollback',id)
    assert placement(first)['settings']==resumed['settings'];assert placement(second)==second_before
    assert not warnings,warnings
    print('PASS: public Start/Pause/Resume via production context, durable acknowledgements, two instances, shared keyboard gear, hide/show/update/rollback')
