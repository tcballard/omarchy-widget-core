#!/usr/bin/env python3
"""Production frame, host handlers and queue under cancelled/stale interactions.

Uses the existing offscreen Qt test stack. Transport is held explicitly so late
replies and queue barriers are deterministic; this does not emulate Wayland.
"""
import os
from pathlib import Path
import tempfile

os.environ.setdefault('QT_QPA_PLATFORM', 'offscreen')
from PySide6.QtCore import QObject, QUrl, QMetaObject, Q_ARG, Q_RETURN_ARG, QPoint, Qt
from PySide6.QtGui import QGuiApplication
from PySide6.QtQml import QQmlApplicationEngine
from PySide6.QtQuick import QQuickWindow
from PySide6.QtTest import QTest

root = Path(__file__).resolve().parents[1]
source = (root/'Host.qml').read_text()
controller = source[source.index('Item {'):source.index('    function screenFor(')]
controller = controller.replace('id: root', 'id: root; objectName:"controller"', 1)
for role, value in [('ROLE', 'manager'), ('REVEAL', '1')]:
    controller = controller.replace(f'Quickshell.env("OMARCHY_WIDGET_{role}") === "{value}"', 'false')
context = source[source.index('            Core.Lifecycle {'):source.index('            Core.WidgetFrame {')]
handlers = source[source.index('                onMoved:'):source.index('                Loader {\n                    id: content')]
geometry_reset = next(line.split('onGeometryKeyChanged: ', 1)[1] for line in source.splitlines() if 'onGeometryKeyChanged:' in line)
app = QGuiApplication([])


def call(obj, name, *args):
    result = QMetaObject.invokeMethod(obj, name, Q_RETURN_ARG('QVariant'),
                                     *[Q_ARG('QVariant', v) for v in args])
    return result.toVariant() if hasattr(result, 'toVariant') else result


with tempfile.TemporaryDirectory() as tmp:
    temp = Path(tmp)
    (temp/'qs').mkdir()
    for name in ['Commons', 'Ui']:
        (temp/'qs'/name).symlink_to(root/name, target_is_directory=True)
    io = temp/'Quickshell/Io'
    io.mkdir(parents=True)
    (io/'qmldir').write_text('module Quickshell.Io\nProcess 1.0 Process.qml\nStdioCollector 1.0 StdioCollector.qml\n')
    (io/'Process.qml').write_text('''import QtQuick
QtObject { property var command:[]; property bool running:false; property QtObject stdout
signal exited(int code, int status) }
''')
    (io/'StdioCollector.qml').write_text('import QtQuick\nQtObject {property string text:"";property bool waitForEnd:false}')
    harness = '''import QtQuick
import qs.Commons
import "CORE" as Core
import "DECLARATIVE" as Declarative
Window {
    id: host; width:520; height:360; visible:true
CONTROLLER
        property var settingsContext: QtObject { property var draftSettings:({}) }
        property var settingsLoader: QtObject { property string source:"" }
        property var fixture: ({instanceId:"weather",directory:"/package/v1",
            manifest:{id:"example.weather",name:"Weather",coreApi:3,families:["small"]},
            placement:{revision:1,settings:{latitude:51.5,longitude:-0.12},workspace:null}})
        function reset() {
            operation.current={args:["held"],token:""}; operation.pending=[];
            root.installed=[fixture];root.editing=true;root.weatherStates={};
            window.positionX=0;window.positionY=0;window.moving=false;window.commits=0;
            frame.visible=true;window.visible=true;
            return true;
        }
        function changeFixture(revision, directory) {
            fixture=Object.assign({},fixture,{directory:directory,placement:{revision:revision,settings:{latitude:40.7,longitude:-74}}});
            installed=[fixture]; return true;
        }
        function queueChecks() {
            operation.enqueue(["place","weather","old"]);
            operation.enqueue(["place","weather","new"]);
            if(operation.pending.length!==1 || operation.pending[0].args[2]!=="new")throw Error("Adjacent moves did not coalesce");
            operation.enqueue(["hide","weather"]);
            operation.enqueue(["place","weather","after-hide"]);
            if(operation.pending.length!==3 || operation.pending[0].args[2]!=="new")throw Error("Move crossed hide barrier");
            operation.pending=[];
            operation.enqueue(["list"]);operation.enqueue(["save","weather","settings"]);operation.enqueue(["list"]);
            if(operation.pending.length!==3)throw Error("Refresh crossed save barrier");
            operation.pending=[];
            operation.enqueue(["weather","weather","51","0"]);
            operation.enqueue(["weather-permission","example.weather","deny"]);
            operation.enqueue(["weather","weather","40","-74"]);
            if(operation.pending.length!==3 || operation.pending[0].args[2]!=="51")throw Error("Weather crossed permission barrier");
            operation.pending=[{args:["list"],token:"retry",retries:4}];
            operation.enqueue(["list"]);
            if(operation.pending.length!==2 || operation.pending[0].retries!==4)throw Error("Retry budget reset by coalescing");
            operation.pending=[];
            for(var i=0;i<64;i++)if(!operation.enqueue(["save",String(i)]))throw Error("Early queue rejection");
            if(operation.enqueue(["save","overflow"]))throw Error("Queue is unbounded");
            operation.pending=[];
            return true;
        }
        function requestWeather() {
            context.requestWeather(context.settings.latitude,context.settings.longitude);
            return operation.pending[operation.pending.length-1];
        }
        function completeWeather(request, success) {
            operation.completed(request,success,{state:"fresh",data:{temperatureC:19}},success ? "" : "old failure");
            return context.weather;
        }
        function weatherState() { return context.weather; }
        function removeWeather() { installed=[];return true; }
        function hideFrame() {frame.visible=false;return true;}
        function hideSurface() {window.visible=false;return true;}
        function finishArrange() {root.editing=false;return true;}
        function cancelForGeometry() {window.geometryReset();return true;}
        function movementState() {return {commits:window.commits,moving:window.moving,x:window.positionX,dragging:frame.dragging};}
        Item {
            id:window; width:480;height:320;visible:true
            property string modelData:"weather"
            property var entry:root.fixture
            property var config:entry.placement
            property var metadata:entry.manifest
            property string sizeName:"small"
            property var effective:({monitor:"eDP-1"})
            property var screen:null
            property bool moving:false
            property bool validTarget:true
            property real positionX:0
            property real positionY:0
            property int commits:0
            function resetPosition(){positionX=0;positionY=0;}
            function geometryReset() GEOMETRY_RESET
            function place(size,monitor){commits++;return true;}
CONTEXT
            Core.WidgetFrame {
                id:frame;anchors.fill:parent;editing:root.editing;title:"Weather"
                onEscapeRequested:root.editing=false
HANDLERS
            }
        }
    }
}
'''.replace('CORE', (root/'qml').as_uri()).replace('DECLARATIVE', (root/'qml/Declarative.js').as_uri()).replace('CONTROLLER', controller).replace('CONTEXT', context).replace('HANDLERS', handlers).replace('GEOMETRY_RESET', geometry_reset)
    path = temp/'Test.qml'
    path.write_text(harness)
    engine = QQmlApplicationEngine()
    engine.addImportPath(str(temp))
    warnings = []
    engine.warnings.connect(lambda errors: warnings.extend(e.toString() for e in errors))
    engine.load(QUrl.fromLocalFile(str(path)))
    assert engine.rootObjects(), warnings
    host = engine.rootObjects()[0]
    # root is the production controller Item; functions are forwarded through it.
    obj = host.findChild(QObject, 'controller')
    assert call(obj, 'reset')
    assert call(obj, 'queueChecks')

    request = call(obj, 'requestWeather')
    assert call(obj, 'completeWeather', request, True)['state'] == 'fresh'
    call(obj, 'changeFixture', 2, '/package/v1')
    assert call(obj, 'weatherState')['data'] is None, 'Old location must disappear immediately'
    assert call(obj, 'completeWeather', request, True)['data'] is None, 'Late old success must be ignored'
    assert call(obj, 'completeWeather', request, False)['state'] == 'loading', 'Late old failure must be ignored'
    request = call(obj, 'requestWeather')
    assert call(obj, 'completeWeather', request, True)['state'] == 'fresh'
    call(obj, 'changeFixture', 2, '/package/v2')
    assert call(obj, 'completeWeather', request, True)['data'] is None, 'Replaced package must reject old response'
    request = call(obj, 'requestWeather')
    call(obj, 'removeWeather')
    assert call(obj, 'completeWeather', request, True)['data'] is None, 'Removed instance must reject late response'

    host.requestActivate()
    QTest.qWait(30)

    def drag():
        call(obj, 'reset')
        QTest.qWait(10)
        QTest.mouseMove(host, QPoint(70, 20), 10)
        QTest.mousePress(host, Qt.LeftButton, Qt.NoModifier, QPoint(70, 20))
        QTest.mouseMove(host, QPoint(130, 20), 10)
        assert call(obj, 'movementState')['moving'], 'Mouse must reach the production drag handler'

    drag()
    QTest.mouseRelease(host, Qt.LeftButton, Qt.NoModifier, QPoint(130, 20))
    assert call(obj, 'movementState')['commits'] == 1
    call(obj, 'reset')
    QTest.mouseClick(host, Qt.LeftButton, Qt.NoModifier, QPoint(70, 20))
    assert call(obj, 'movementState')['commits'] == 0, 'A click is not a move'
    other = QQuickWindow()
    pointer = host.findChild(QObject, 'placement-drag')
    for cancel in ['pointerCancel', 'hideFrame', 'hideSurface', 'finishArrange', 'cancelForGeometry', 'escape', 'unmap', 'blur']:
        drag()
        if cancel == 'pointerCancel':
            pointer.ungrabMouse()
        elif cancel == 'escape':
            QTest.keyClick(host, Qt.Key_Escape)
        elif cancel == 'unmap':
            host.hide()
            QTest.qWait(20)
        elif cancel == 'blur':
            other.show()
            other.requestActivate()
            QTest.qWait(20)
        else:
            call(obj, cancel)
        QTest.mouseRelease(host, Qt.LeftButton, Qt.NoModifier, QPoint(130, 20))
        state = call(obj, 'movementState')
        assert state == dict(commits=0, moving=False, x=0, dragging=False), (cancel, state)
        other.hide()
        host.show()
        host.requestActivate()
        QTest.qWait(20)
    call(obj, 'reset')
    QTest.mouseClick(host, Qt.LeftButton, Qt.NoModifier, QPoint(70, 20))
    QTest.keyClick(host, Qt.Key_Right)
    assert call(obj, 'movementState')['commits'] == 1, 'Keyboard placement remains supported'
    drag()
    QTest.keyClick(host, Qt.Key_Right)
    QTest.mouseRelease(host, Qt.LeftButton, Qt.NoModifier, QPoint(130, 20))
    assert call(obj, 'movementState') == dict(commits=1, moving=False, x=0, dragging=False), 'Keyboard takes over a drag without a second commit'
    assert not warnings, warnings
print('PASS: cancelled drags never commit; stale weather replies are discarded; queue coalescing preserves barriers and retry bounds')
