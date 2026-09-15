#!/usr/bin/env python3
"""Native Qt component test with explicit Quattro token/button fixtures.
Does not emulate the Wayland compositor or claim live Omarchy validation.
"""
import json, os, subprocess, sys, tempfile
from pathlib import Path
os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")
from PySide6.QtCore import QUrl, QTimer, QObject, QMetaObject, Qt
from PySide6.QtGui import QGuiApplication
from PySide6.QtQml import QQmlApplicationEngine
from PySide6.QtQuick import QQuickWindow
from PySide6.QtTest import QTest
import PySide6
root = Path(__file__).resolve().parents[1]
formatter = Path(PySide6.__file__).parent / "qmlformat"
for path in [root / "Service.qml", *sorted((root / "qml").glob("*.qml"))]:
    subprocess.run([str(formatter), str(path)], check=True, stdout=subprocess.DEVNULL)
stubs = {'qs/Ui/Button.qml': 'import QtQuick\nimport qs.Commons\nRectangle {\n id:root\n property string text:""\n property string tooltipText:""\n property bool selected:false\n property bool focusable:false\n property real fontSize:Style.font.body\n property real horizontalPadding:Style.space(9)\n signal clicked()\n implicitWidth:label.implicitWidth+horizontalPadding*2\n implicitHeight:label.implicitHeight+Style.space(14)\n color:selected?Qt.rgba(Color.accent.r,Color.accent.g,Color.accent.b,.12):"transparent"\n border.width:activeFocus?1:0;border.color:Color.accent\n activeFocusOnTab:focusable\n Keys.onReturnPressed:clicked()\n Keys.onSpacePressed:clicked()\n Text {id:label;anchors.centerIn:parent;textFormat:Text.PlainText;text:root.text;color:root.selected?Color.accent:Color.foreground;font.family:Style.font.family;font.pixelSize:root.fontSize}\n MouseArea {anchors.fill:parent;onClicked:{if(root.focusable)root.forceActiveFocus();root.clicked()}}\n}\n', 'qs/Ui/qmldir': 'module qs.Ui\nButton 1.0 Button.qml\n', 'qs/Commons/Style.qml': 'pragma Singleton\nimport QtQuick\nQtObject {\n property real scale:1\n property int cornerRadius:0\n function spaceReal(n){return n*scale}\n function space(n){return n*scale}\n property QtObject font:QtObject {\n  property string family:"DejaVu Sans Mono"\n  property real body:12*Style.scale\n  property real bodySmall:10*Style.scale\n  property real heading:16*Style.scale\n }\n}\n', 'qs/Commons/Color.qml': 'pragma Singleton\nimport QtQuick\nQtObject {\n property bool light:false\n property color foreground:light?"#263022":"#e4e8df"\n property color background:light?"#f1f0e8":"#171c1a"\n property color accent:light?"#526c36":"#b3cb92"\n property color muted:light?"#68715e":"#8a9588"\n property color urgent:light?"#9c3e30":"#e5a085"\n}\n', 'qs/Commons/qmldir': 'module qs.Commons\nsingleton Color 1.0 Color.qml\nsingleton Style 1.0 Style.qml\n'}
with tempfile.TemporaryDirectory() as tmp:
    temp = Path(tmp)
    for name, text in stubs.items():
        path = temp / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)
    preview = temp / "Preview.qml"
    preview.write_text('''import QtQuick
import qs.Commons
import "''' + (root / "qml").as_uri() + '''" as Core
Window {
    width: 540; height: 500; visible: true
    Core.Manager {
        id: manager; objectName: "manager"; anchors.fill: parent
        onCloseRequested: closed = true
        property bool closed: false
    }
    function populate() {
        manager.entries = [{manifest:{id:"io.example.fixture",name:"Contract fixture",version:"0.1.0"},placement:null}]
    }
    Core.WidgetFrame {
        id:frame; objectName:"frame"; anchors.fill:parent; anchors.margins:20; visible:false
        appearance:({radius:16,borderAlpha:0.08,fontFamily:"DejaVu Sans"})
        title:"Fixture"; onEditRequested:editing=!editing
        Core.Label { anchors.centerIn:parent;text:"Theme-aware widget frame" }
    }
    function lightFrame() { manager.visible=false; frame.visible=true; Color.light=true; }
    function darkFrame() { Color.light=false; frame.editing=true; }
}
''')
    app = QGuiApplication([])
    engine = QQmlApplicationEngine()
    engine.addImportPath(tmp)
    warnings = []
    engine.warnings.connect(lambda errors: warnings.extend(e.toString() for e in errors))
    engine.load(QUrl.fromLocalFile(str(preview)))
    assert engine.rootObjects(), "Manager failed to load"
    def check():
        try:
            window = engine.rootObjects()[0]
            manager = window.findChild(QObject, "manager")
            assert manager is not None
            QMetaObject.invokeMethod(window, "populate")
            app.processEvents()
            QMetaObject.invokeMethod(manager, "forceActiveFocus")
            QTest.keyClick(window, Qt.Key.Key_Escape)
            assert manager.property("closed"), "Escape did not close manager"
            out = root / "test-results" / "manager.png"
            out.parent.mkdir(exist_ok=True)
            assert window.grabWindow().save(str(out))
            QMetaObject.invokeMethod(window,"lightFrame")
            app.processEvents()
            surface=window.findChild(QObject,"widget-surface")
            assert surface.property("radius")==16
            edit=window.findChild(QObject,"edit-button")
            assert not edit.property("visible"), "Idle title/control must be hidden"
            assert window.grabWindow().save(str(out.parent/"frame-light.png"))
            QMetaObject.invokeMethod(window,"darkFrame")
            app.processEvents()
            assert edit.property("visible"), "Arrange controls must remain available"
            assert window.grabWindow().save(str(out.parent/"frame-dark-edit.png"))
            assert not warnings, "\n".join(warnings)
            print("PASS: QML parses; manager lifecycle; soft light frame; dark arrangement controls")
            app.exit(0)
        except Exception:
            import traceback
            traceback.print_exc()
            app.exit(1)
    QTimer.singleShot(300, check)
    sys.exit(app.exec())
