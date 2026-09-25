import QtQuick
import qs.Commons

Rectangle {
    id: root
    property string text: ""
    property string accessibleText: text
    property bool selected: false
    property string tone: "normal"
    signal clicked()
    readonly property color ink: tone === "danger" ? Color.urgent : Color.foreground
    implicitWidth: label.implicitWidth + Style.space(24)
    implicitHeight: Style.space(36)
    radius: Math.min(Style.cornerRadius, Style.space(8))
    color: selected || tone === "primary"
        ? Qt.rgba(Color.accent.r, Color.accent.g, Color.accent.b, mouse.containsMouse ? 0.22 : 0.13)
        : Qt.rgba(ink.r, ink.g, ink.b, mouse.containsMouse ? 0.09 : tone === "quiet" ? 0 : 0.04)
    border.width: activeFocus || selected || tone === "primary" ? 1 : 0
    border.color: activeFocus ? Color.accent : Qt.rgba(Color.accent.r, Color.accent.g, Color.accent.b, 0.35)
    opacity: enabled ? 1 : 0.45
    activeFocusOnTab: true
    onActiveFocusChanged: if (activeFocus) {
        for (var p = root.parent; p; p = p.parent)
            if (typeof p.ensureVisible === "function") { p.ensureVisible(root); break; }
    }
    Accessible.role: Accessible.Button
    Accessible.name: accessibleText
    Accessible.onPressAction: if (enabled) clicked()
    Keys.onReturnPressed: if (enabled) clicked()
    Keys.onSpacePressed: if (enabled) clicked()
    Text {
        id: label
        anchors.centerIn: parent
        text: root.text
        textFormat: Text.PlainText
        color: root.ink
        font.family: Style.font.family
        font.pixelSize: Style.space(13)
        font.weight: root.selected || root.tone === "primary" ? Font.DemiBold : Font.Normal
    }
    MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: { root.forceActiveFocus(); root.clicked(); }
    }
}
