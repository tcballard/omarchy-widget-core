import QtQuick
import QtQuick.Layouts
import qs.Commons
import qs.Ui as Ui

FocusScope {
    id: root
    property bool busy: false
    property string error: ""
    default property alias contents: slot.data
    signal cancelRequested()
    signal saveRequested()
    focus: true
    Keys.onEscapePressed: if(!busy) cancelRequested()
    Rectangle { anchors.fill:parent; color:Color.background; radius:Style.space(16) }
    ColumnLayout {
        anchors.fill:parent; anchors.margins:Style.space(20); spacing:Style.space(12)
        Label { text:"Widget settings"; font.pixelSize:Style.font.heading; font.bold:true }
        Item { id:slot; Layout.fillWidth:true; Layout.fillHeight:true; clip:true; enabled:!root.busy }
        Label { text:root.error; visible:text!==""; wrapMode:Text.Wrap; color:Color.urgent; Layout.fillWidth:true }
        RowLayout {
            Layout.fillWidth:true
            Ui.Button { text:"Cancel"; enabled:!root.busy; focusable:true; onClicked:root.cancelRequested() }
            Item { Layout.fillWidth:true }
            Ui.Button { text:root.busy?"Saving…":"Save"; enabled:!root.busy; focusable:true; onClicked:root.saveRequested() }
        }
    }
}
