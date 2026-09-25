#!/usr/bin/env python3
"""Production shared-role controller + generated editor + real CLI acknowledgements."""
import json,os,subprocess,sys,tempfile,time
from pathlib import Path
os.environ.setdefault('QT_QPA_PLATFORM','offscreen')
from PySide6.QtCore import QObject,QUrl,QMetaObject,Q_ARG,Q_RETURN_ARG,Qt
from PySide6.QtGui import QGuiApplication
from PySide6.QtQuick import QQuickWindow
from PySide6.QtQml import QQmlApplicationEngine,qmlRegisterType
from PySide6.QtTest import QTest
from qt_process import Process
root=Path(__file__).resolve().parents[1];binary=Path(sys.argv[1]).resolve()
qmlRegisterType(Process,'Quickshell.Io',1,0,'Process');app=QGuiApplication([])
def invoke(obj,name,*args):
    return QMetaObject.invokeMethod(obj,name,Q_RETURN_ARG('QVariant'),*[Q_ARG('QVariant',arg) for arg in args])
def wait(condition):
    until=time.monotonic()+10
    while time.monotonic()<until:
        app.processEvents()
        if condition():return
        QTest.qWait(10)
    raise AssertionError('Controller did not settle')
with tempfile.TemporaryDirectory() as d:
    temp=Path(d);os.environ['XDG_DATA_HOME']=str(temp/'data');os.environ['XDG_STATE_HOME']=str(temp/'state')
    def cli(*args):return json.loads(subprocess.check_output([str(binary),*args],text=True))
    cli('install',str(root/'examples/declarative-clock'));instance=cli('create','io.github.tcballard.worldclock','small')['updated']
    for name in ['Commons','Ui']:
        (temp/'qs').mkdir(exist_ok=True);(temp/'qs'/name).symlink_to(root/name,target_is_directory=True)
    (temp/'qml').symlink_to(root/'qml',target_is_directory=True)
    io=temp/'Quickshell/Io';io.mkdir(parents=True)
    (io/'qmldir').write_text('module Quickshell.Io\nStdioCollector 1.0 StdioCollector.qml\n')
    (io/'StdioCollector.qml').write_text('import QtQuick\nQtObject {property string text:"";property bool waitForEnd:false}')
    source=(root/'Host.qml').read_text()
    controller=source[source.index('Item {'):source.index('    function screenFor(')]
    controller=controller.replace('id: root','id: root;objectName:"controller"',1).replace('Quickshell.env("OMARCHY_WIDGET_ROLE") === "manager"','true').replace('Quickshell.env("OMARCHY_WIDGET_REVEAL") === "1"','false')
    helper=next(line for line in controller.splitlines() if 'readonly property string helper:' in line)
    controller=controller.replace(helper,'property string helper:'+json.dumps(str(binary)))
    context=source[source.index('    QtObject {\n        id: settingsContext'):source.index('    FloatingWindow {\n        title: "Widget settings"')]
    panel=source[source.index('        Core.SettingsPanel {'):source.index('    Variants {')].rsplit('\n    }',1)[0]
    fixture=temp/'Test.qml';fixture.write_text('''import QtQuick
import qs.Commons
import "CORE" as Core
import "HELPER" as Declarative
Window {width:520;height:560;visible:true
CONTROLLER
CONTEXT
PANEL
function sync() {return refresh();}
function pending() {return operation.busy;}
function current() {return configuring;}
function open() {configure(INSTANCE);return true;}
function change() {settingsLoader.item.change("displayMode","analogue");return true;}
function draft() {return settingsContext.draftSettings.displayMode;}
function close() {closeSettings();return true;}
function commit() {return save(configuring,settingsContext.draftSettings,settingsContext.revision);}
Component.onCompleted:refresh()
}}
'''.replace('CORE',(root/'qml').as_uri()).replace('HELPER',(root/'qml/Declarative.js').as_uri()).replace('CONTROLLER',controller).replace('CONTEXT',context).replace('PANEL',panel).replace('INSTANCE',json.dumps(instance)))
    engine=QQmlApplicationEngine();engine.addImportPath(str(temp));warnings=[];engine.warnings.connect(lambda es:warnings.extend(e.toString() for e in es));engine.load(QUrl.fromLocalFile(str(fixture)));assert engine.rootObjects(),warnings
    window=engine.rootObjects()[0];obj=window.findChild(QObject,'controller');wait(lambda:not invoke(obj,'pending'))
    def saved():return cli('list')['installed'][0]['placement']['settings']['displayMode']
    invoke(obj,'open');wait(lambda:invoke(obj,'current')==instance and not invoke(obj,'pending'))
    invoke(obj,'change');assert invoke(obj,'draft')=='analogue' and saved()=='digital'
    invoke(obj,'close');wait(lambda:not invoke(obj,'pending'));assert saved()=='digital' and cli('list')['runtime']['edit'] is None
    invoke(obj,'open');wait(lambda:invoke(obj,'current')==instance and not invoke(obj,'pending'))
    invoke(obj,'change');assert invoke(obj,'commit');wait(lambda:invoke(obj,'current')=='' and not invoke(obj,'pending'))
    assert saved()=='analogue' and cli('list')['runtime']['edit'] is None
    invoke(obj,'open');wait(lambda:invoke(obj,'current')==instance and not invoke(obj,'pending'));assert invoke(obj,'draft')=='analogue'
    invoke(obj,'close');wait(lambda:not invoke(obj,'pending'));assert not warnings,warnings
print('PASS: shared-role production controller opens Core-only editor; Cancel preserves disk, Save acknowledges/closes, reopen restores values')
