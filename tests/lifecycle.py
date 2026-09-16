#!/usr/bin/env python3
"""Exercise the actual lifecycle component and weather editor under offscreen Qt."""
import os,tempfile,sys
from pathlib import Path
os.environ['QT_QPA_PLATFORM']='offscreen'
from PySide6.QtGui import QGuiApplication
from PySide6.QtCore import QUrl
from PySide6.QtQml import QQmlApplicationEngine
root=Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory() as tmp:
    p=Path(tmp);(p/'qs').mkdir();(p/'qs'/'Commons').symlink_to(root/'Commons',target_is_directory=True)
    source='''import QtQuick
import "CORE" as Core
import "WEATHER" as Weather
Item {
    property var events:[]
    Core.Lifecycle {
        id:life
        onResuming:events.push("resume")
        onBecameVisible:events.push("visible")
        onBecameHidden:events.push("hidden")
        onSuspending:events.push("suspend")
    }
    QtObject {id:settings;property var draftSettings:({latitude:51.5,longitude:-0.12,label:"London"})}
    Weather.Settings {id:editor;settingsContext:settings}
    function check() {
        life.active=true;life.active=false;life.active=true;
        if(events.join(",")!=="resume,visible,hidden,suspend,resume,visible")throw Error("Lifecycle order");
        if(!life.backgroundAllowed || life.lifecycle!=="visible")throw Error("Lifecycle state");
        if(editor.validationError)throw Error(editor.validationError);
        settings.draftSettings={latitude:100,longitude:0,label:"Invalid"};
        if(!editor.validationError)throw Error("Invalid coordinates accepted");
    }
}
'''.replace('CORE',(root/'qml').as_uri()).replace('WEATHER',(root/'examples/weather').as_uri())
    (p/'Test.qml').write_text(source)
    app=QGuiApplication([]);engine=QQmlApplicationEngine();engine.addImportPath(tmp)
    warnings=[];engine.warnings.connect(lambda errors:warnings.extend(e.toString() for e in errors))
    engine.load(QUrl.fromLocalFile(str(p/'Test.qml')))
    assert engine.rootObjects()
    from PySide6.QtCore import QMetaObject
    QMetaObject.invokeMethod(engine.rootObjects()[0],'check')
    assert not warnings,'\n'.join(warnings)
print('PASS: lifecycle transition order, activity gates and weather settings validation')
