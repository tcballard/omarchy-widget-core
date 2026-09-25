#!/usr/bin/env python3
"""Render only the bundled, trusted starter. Does not load an arbitrary widget package."""
import json,os,sys,tempfile
from pathlib import Path
os.environ['QT_QPA_PLATFORM']='offscreen'
from PySide6.QtQuick import QQuickWindow
from PySide6.QtGui import QGuiApplication
from PySide6.QtCore import QUrl,QTimer,QMetaObject
from PySide6.QtQml import QQmlApplicationEngine
root=Path(__file__).resolve().parents[1]
output=Path(sys.argv[1]).resolve();output.mkdir(parents=True,exist_ok=True)
with tempfile.TemporaryDirectory() as tmp:
    p=Path(tmp);(p/'qs').mkdir()
    for name in ['Commons','Ui']:(p/'qs'/name).symlink_to(root/name,target_is_directory=True)
    qml='''import QtQuick
import qs.Commons
import "CORE" as Core
Window {
    id:window; width:192;height:192;visible:true;color:Color.background
    property string family:"small"
    QtObject {
        id:context
        readonly property var theme:Color
        readonly property var metrics:Style
        readonly property var settings:({title:"Today",note:"Choose one thing to focus on.\\nLeave room to think."})
        readonly property int settingsRevision:1
        readonly property string family:window.family
        readonly property bool active:true
    }
    Core.WidgetFrame {
        anchors.fill:parent; title:"Widget starter"
        appearance:({radius:12,borderWidth:1,fontFamily:"DejaVu Sans"})
        Loader {
            anchors.fill:parent
            Component.onCompleted:setSource(VIEW,{widgetContext:context})
        }
    }
    Component.onCompleted:Style.fontFamily="DejaVu Sans"
}
'''.replace('CORE',(root/'qml').as_uri()).replace('VIEW',json.dumps((root/'sdk/starter/View.qml').as_uri()))
    (p/'Preview.qml').write_text(qml)
    app=QGuiApplication([]);engine=QQmlApplicationEngine();engine.addImportPath(tmp)
    warnings=[];engine.warnings.connect(lambda errors:warnings.extend(e.toString() for e in errors))
    engine.load(QUrl.fromLocalFile(str(p/'Preview.qml')));assert engine.rootObjects()
    window=engine.rootObjects()[0];sizes=[('small',192,192),('medium',394,192),('large',394,394)]
    def render(index=0):
        try:
            assert not warnings,'\n'.join(warnings)
            name,w,h=sizes[index];picture=window.grabWindow()
            assert not picture.isNull() and picture.width()==w and picture.height()==h
            assert picture.save(str(output/(name+'.png')))
            if index==2:app.exit(0);return
            name,w,h=sizes[index+1];window.setProperty('family',name);window.resize(w,h)
            QTimer.singleShot(150,lambda:render(index+1))
        except Exception:
            import traceback;traceback.print_exc();app.exit(1)
    QTimer.singleShot(200,render);code=app.exec()
    if code:sys.exit(code)
print('PASS: rendered bundled starter in Small, Medium and Large')
