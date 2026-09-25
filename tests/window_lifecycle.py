#!/usr/bin/env python3
"""Production binding/exit conditions with a window-screen feedback fixture.
No real compositor claim: desktop hide/show and focus still require live checks.
"""
import os
from pathlib import Path
import tempfile
os.environ.setdefault('QT_QPA_PLATFORM', 'offscreen')
from PySide6.QtCore import QUrl, QMetaObject
from PySide6.QtGui import QGuiApplication
from PySide6.QtQml import QQmlApplicationEngine
from PySide6.QtTest import QTest

root = Path(__file__).resolve().parents[1]
source = (root/'Host.qml').read_text()
visible = next(line.strip().split('visible: ',1)[1] for line in source.splitlines()
               if 'visible: (root.shown || root.revealRunner)' in line)
selected = next(line.strip() for line in source.splitlines() if 'readonly property var selectedScreen:' in line)
exit_condition = next(line.strip().split('running:',1)[1] for line in source.splitlines()
                      if 'running:root.managerRole && !root.declarativeWanted' in line)
app = QGuiApplication([])
with tempfile.TemporaryDirectory() as temp:
    path = Path(temp)/'Test.qml'
    path.write_text('''import QtQuick
import "'''+(root/'qml/Workspace.js').as_uri()+'''" as Workspace
Item {
    id: root
    property bool shown: true
    property bool revealRunner: false
    property var desktop: ({available:true,monitors:{"eDP-1":1}})
    property var monitor: ({name:"eDP-1"})
    property bool monitorAvailable: true
    function screenFor(name) { return monitorAvailable ? monitor : null; }
    function clearPending() { pendingClose=null; }
    property bool declarativeWanted: false
    property string configuring: ""
    property bool managerRole: true
    property bool snapshotReady: false
    property bool managerOpen: false
    property bool editing: false
    property bool revealing: false
    property var pendingClose: null
    property var contentFailures: ({})
    property bool queueBusy: false
    property int quitCount: 0
    QtObject { id: operation; readonly property bool busy: root.queueBusy }
    Timer { interval:250; running:'''+exit_condition+'''; onTriggered:root.quitCount++ }
    Item {
        id: surface
        property var effective: ({monitor:"eDP-1"})
        property var config: ({workspace:null})
        '''+selected+'''
        // Emulate screen becoming unavailable when a native window is unmapped.
        property var screen: visible ? selectedScreen : null
        visible: '''+visible+'''
    }
    readonly property bool surfaceVisible: surface.visible
}
''')
    engine = QQmlApplicationEngine()
    warnings = []
    engine.warnings.connect(lambda errors:warnings.extend(e.toString() for e in errors))
    engine.load(QUrl.fromLocalFile(str(path)))
    assert engine.rootObjects(), warnings
    obj = engine.rootObjects()[0]
    assert obj.property('surfaceVisible')
    obj.setProperty('shown',False);QTest.qWait(10)
    assert not obj.property('surfaceVisible')
    obj.setProperty('shown',True);QTest.qWait(10)
    assert obj.property('surfaceVisible')
    obj.setProperty('monitorAvailable',False);QTest.qWait(10)
    assert not obj.property('surfaceVisible')
    obj.setProperty('monitorAvailable',True);QTest.qWait(10)
    assert obj.property('surfaceVisible')
    QTest.qWait(300)
    assert obj.property('quitCount')==0, 'Must receive first snapshot before exiting'
    obj.setProperty('snapshotReady',True)
    for reason in ['queueBusy','managerOpen','editing','revealing','declarativeWanted']:
        obj.setProperty(reason,True);QTest.qWait(300)
        assert obj.property('quitCount')==0, reason
        obj.setProperty(reason,False)
    obj.setProperty('pendingClose',{'instance':'clock'});QTest.qWait(300)
    assert obj.property('quitCount')==0
    QMetaObject.invokeMethod(obj,'clearPending');QTest.qWait(350)
    assert obj.property('quitCount')==1, 'Idle manager should release its process'
    assert not warnings, warnings
print('PASS: visibility does not depend on mapped screen; manager waits for surfaces and queued actions before exit')
