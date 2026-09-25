import QtQuick
import qs.Commons

// Static package artwork only. Never load widget QML into the manager.
Item {
    id: root
    property string family: "small"
    property url imageSource: ""
    readonly property int columns: family === "small" ? 1 : 2
    readonly property int rows: family === "large" ? 2 : 1
    Image {
        id: preview
        anchors.fill: parent
        source: root.imageSource
        sourceSize.width: 512; sourceSize.height: 256
        fillMode: Image.PreserveAspectFit
        asynchronous: true
        visible: status === Image.Ready
    }
    Column {
        anchors.centerIn: parent
        visible: preview.status !== Image.Ready
        spacing: Style.space(8)
        Item {
            width: Style.space(116); height: Style.space(68)
            Rectangle {
                anchors.centerIn: parent
                width: Style.space(root.columns * 30 + (root.columns - 1) * 4)
                height: Style.space(root.rows * 30 + (root.rows - 1) * 4)
                color: "transparent"; border.color: Color.accent; border.width: 1
                Grid {
                    anchors.fill: parent; columns: root.columns; spacing: Style.space(4)
                    Repeater {
                        model: root.columns * root.rows
                        Rectangle { width: Style.space(30); height: width; color: Qt.rgba(Color.accent.r,Color.accent.g,Color.accent.b,0.14) }
                    }
                }
            }
        }
        Label { width: Style.space(116); horizontalAlignment: Text.AlignHCenter; text: root.columns + " × " + root.rows + " cells"; color: Color.muted; font.pixelSize: Style.font.bodySmall }
    }
}
