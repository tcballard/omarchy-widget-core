#!/usr/bin/env python3
"""Production GitHub QML with deterministic broker data; offscreen, not Hyprland."""
import datetime, json, os, sys, tempfile
from pathlib import Path
os.environ.setdefault('QT_QPA_PLATFORM','offscreen')
os.environ.setdefault('QT_QUICK_BACKEND','software')
from PySide6.QtCore import QUrl, QMetaObject, QObject, Qt, Q_ARG, QPointF
from PySide6.QtGui import QGuiApplication
from PySide6.QtQuick import QQuickWindow
from PySide6.QtQml import QQmlApplicationEngine
from PySide6.QtTest import QTest
root=Path(__file__).resolve().parents[1]
package=root/'widgets/github-contributions'
start=datetime.date(2025,10,5)
days=[dict(date=str(start+datetime.timedelta(days=i)),weekday=(i%7),count=i%5,level=i%5) for i in range(366)]
data=dict(username='tcballard',days=days,total=sum(d['count'] for d in days))
if len(sys.argv)>1: data=json.loads(Path(sys.argv[1]).read_text())
output=Path(sys.argv[2]) if len(sys.argv)>2 else root/'test-results/github-widget'
output.mkdir(parents=True,exist_ok=True)
app=QGuiApplication([])
with tempfile.TemporaryDirectory() as tmp:
    p=Path(tmp);(p/'qs').mkdir()
    for name in ['Commons','Ui']:(p/'qs'/name).symlink_to(root/name,target_is_directory=True)
    qml='''import QtQuick
import qs.Commons
import "CORE" as Core
import "PACKAGE" as Github
Window {
    id:window;width:394;height:192;visible:true;color:Color.background
    property string family:"medium"
    Core.Lifecycle {
        id:context;objectName:"context";active:true
        property var settings:({username:"tcballard",palette:"GitHub green"})
        property var github:({state:"ready",data:CALENDAR,updatedAt:1,error:""})
        property var theme:Color
        property var metrics:Style
        property string family:window.family
        property int requests:0
        property string requestedUser:""
        function requestGithub(username) {requests++;requestedUser=username;return true;}
    }
    Core.WidgetFrame {
        id:frame;anchors.fill:parent;configurable:true;title:"GitHub Contributions"
        appearance:({radius:12,borderWidth:1,fontFamily:"DejaVu Sans"})
        Github.View {id:view;objectName:"github-view";anchors.fill:parent;widgetContext:context}
    }
    QtObject {
        id:settingsContext;objectName:"settings-context"
        property var draftSettings:({username:"tcballard",palette:"GitHub green"})
        property var theme:Color
        property var metrics:Style
    }
    Github.Settings {id:editor;objectName:"editor";visible:false;width:400;height:240;settingsContext:settingsContext}
    function inactive(){context.active=false;view.refresh();}
    function changeUser(){context.settings={username:"octocat",palette:"Theme accent"};}
    function resume(){context.active=true;view.refresh();}
    function invalid(){settingsContext.draftSettings={username:"--bad",palette:"GitHub green"};}
    function light(){Color.apply({foreground:"#202622",background:"#f1f4ee",accent:"#325d40"});}
    function state(name){context.github={state:name,data:name==="stale" ? CALENDAR : null,error:"Provider unavailable"};}
}
'''.replace('CORE',(root/'qml').as_uri()).replace('PACKAGE',package.as_uri()).replace('CALENDAR',json.dumps(data))
    (p/'Test.qml').write_text(qml)
    engine=QQmlApplicationEngine();engine.addImportPath(tmp)
    warnings=[];engine.warnings.connect(lambda errors:warnings.extend(e.toString() for e in errors))
    engine.load(QUrl.fromLocalFile(str(p/'Test.qml')));assert engine.rootObjects()
    window=engine.rootObjects()[0];context=window.findChild(QObject,'context');editor=window.findChild(QObject,'editor')
    def call(name):assert QMetaObject.invokeMethod(window,name)
    for family,width,height in [('small',192,192),('medium',394,192),('large',394,394)]:
        window.setProperty('family',family);window.resize(width,height);QTest.qWait(120)
        assert not warnings,'\n'.join(warnings)
        view=window.findChild(QObject,'github-view')
        def check_bounds(item):
            for child in item.childItems():
                if child.isVisible():
                    if child.metaObject().className().startswith('QQuickText'):
                        point=child.mapToItem(view,QPointF(0,0))
                        assert point.y()+child.height()<=view.height()+1, (family,child.property('text'),point.y(),child.height(),view.height(),'clipped')
                    check_bounds(child)
        check_bounds(view)
        assert window.grabWindow().save(str(output/(family+'.png')))
    assert context.property('requests')>=1
    before=context.property('requests');call('inactive');call('changeUser');QTest.qWait(10)
    assert context.property('requests')==before,'Hidden widget fetched'
    call('resume');QTest.qWait(10);assert context.property('requestedUser')=='octocat'
    assert editor.property('validationError')==''
    call('invalid');assert editor.property('validationError')
    call('light');QTest.qWait(50);assert window.grabWindow().save(str(output/'large-light.png'))
    for state in ['loading','unavailable','stale']:
        QMetaObject.invokeMethod(window,'state',Q_ARG('QVariant',state));QTest.qWait(50)
        assert window.grabWindow().save(str(output/(state+'.png')))
    assert not warnings,'\n'.join(warnings)
print('PASS: GitHub real QML, all families, light/dark, loading/error/stale, settings validation and inactive/resume gating')
