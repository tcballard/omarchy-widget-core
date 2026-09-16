import QtQuick
import qs.Commons

Rectangle {
    id: root
    property string text: ""
    property string tooltipText: ""
    property bool selected: false
    property bool focusable: true
    property real fontSize: Style.font.body
    property real horizontalPadding: Style.space(10)
    signal clicked()
    implicitWidth: label.implicitWidth+horizontalPadding*2
    implicitHeight: Style.space(32)
    radius: Style.space(7)
    color: Qt.rgba(Color.foreground.r,Color.foreground.g,Color.foreground.b,mouse.containsMouse?0.1:0.04)
    border.width: activeFocus?1:0
    border.color: Color.accent
    opacity: enabled?1:0.45
    activeFocusOnTab: focusable
    onActiveFocusChanged: if(activeFocus) Qt.callLater(function() {
        for(var p=root.parent;p;p=p.parent) if(typeof p.ensureVisible === "function") { p.ensureVisible(root); break; }
    })
    Accessible.role: Accessible.Button
    Accessible.name: text
    Accessible.onPressAction: if(enabled) clicked()
    Keys.onReturnPressed: if(enabled) clicked()
    Keys.onSpacePressed: if(enabled) clicked()
    Text {
        id:label; anchors.centerIn:parent; text:root.text; textFormat:Text.PlainText
        color:root.selected?Color.accent:Color.foreground
        font.family:Style.font.family; font.pixelSize:root.fontSize
    }
    MouseArea {
        id:mouse; anchors.fill:parent; hoverEnabled:true
        cursorShape:Qt.PointingHandCursor
        onClicked: { if(root.focusable)root.forceActiveFocus();root.clicked(); }
    }
}
