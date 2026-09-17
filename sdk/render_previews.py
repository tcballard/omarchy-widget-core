#!/usr/bin/env python3
"""Trusted capture worker. Use capture-package for any third-party package."""
import json,os,sys,tempfile
from pathlib import Path
os.environ['QT_QPA_PLATFORM']='offscreen'
os.environ.setdefault('QT_QUICK_BACKEND','software')
from PySide6.QtQuick import QQuickWindow
from PySide6.QtGui import QGuiApplication
from PySide6.QtCore import QUrl,QTimer,QMetaObject
from PySide6.QtQml import QQmlApplicationEngine
root=Path(__file__).resolve().parents[1]
package=Path(sys.argv[3]).resolve() if len(sys.argv)==4 and sys.argv[2]=='--package' else root/'sdk/starter'
manifest=json.loads((package/'widget.json').read_text())
entry=(package/manifest['entryPoint']).resolve();assert entry.is_relative_to(package.resolve())
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
    Core.Lifecycle {
        id:context
        readonly property int api:3
        readonly property string instanceId:"preview-instance"
        readonly property string packageId:PACKAGE_ID
        readonly property string definitionId:"main"
        readonly property string sizeName:window.family
        readonly property int draftRevision:settingsRevision
        readonly property bool inputRequested:false
        readonly property var weather:({state:"unavailable",data:null,error:"Preview has no network access"})
        readonly property var saveState:({saving:false,saved:false,error:""})
        function requestWeather(latitude,longitude) { return false; }
        function requestInput(enabled) { return false; }
        readonly property var theme:Color
        readonly property var metrics:Style
        readonly property var settings:DEFAULTS
        readonly property int settingsRevision:1
        readonly property string family:window.family
        active:false
        readonly property bool saving:false
        readonly property bool saved:false
        readonly property string saveError:""
        readonly property var appearance:({})
        function requestConfigure() {}
        function saveSettings(value,revision) { return false; }
    }
    Core.WidgetFrame {
        anchors.fill:parent; title:NAME; configurable:CONFIGURABLE
        appearance:({radius:12,borderWidth:1,fontFamily:"DejaVu Sans"})
        Loader {
            anchors.fill:parent
            Component.onCompleted:setSource(VIEW,{widgetContext:context})
        }
    }
    Component.onCompleted:Style.fontFamily="DejaVu Sans"
}
'''.replace('PACKAGE_ID',json.dumps(manifest['id'])).replace('CORE',(root/'qml').as_uri()).replace('VIEW',json.dumps(entry.as_uri())).replace('DEFAULTS',json.dumps(manifest['defaults'])).replace('NAME',json.dumps(manifest['name'])).replace('CONFIGURABLE','true' if manifest.get('settingsEntryPoint') else 'false')
    (p/'Preview.qml').write_text(qml)
    app=QGuiApplication([]);engine=QQmlApplicationEngine();engine.addImportPath(tmp);engine.addImportPath(str(root/'sdk/preview-imports'))
    warnings=[];engine.warnings.connect(lambda errors:warnings.extend(e.toString() for e in errors))
    engine.load(QUrl.fromLocalFile(str(p/'Preview.qml')));assert engine.rootObjects()
    window=engine.rootObjects()[0];sizes=[v for v in [('small',192,192),('medium',394,192),('large',394,394)] if v[0] in manifest['families']]
    window.setProperty('family',sizes[0][0]);window.resize(sizes[0][1],sizes[0][2])
    def render(index=0):
        try:
            assert not warnings,'\n'.join(warnings)
            name,w,h=sizes[index];picture=window.grabWindow()
            assert not picture.isNull() and picture.width()==w and picture.height()==h
            assert picture.save(str(output/(name+'.png')))
            if index==len(sizes)-1:app.exit(0);return
            name,w,h=sizes[index+1];window.setProperty('family',name);window.resize(w,h)
            QTimer.singleShot(150,lambda:render(index+1))
        except Exception:
            import traceback;traceback.print_exc();app.exit(1)
    QTimer.singleShot(200,render);code=app.exec()
    if code:sys.exit(code)
print('PASS: rendered '+manifest['name']+' in '+', '.join(v[0] for v in sizes))
