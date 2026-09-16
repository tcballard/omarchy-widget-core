import QtQuick
import QtQuick.Layouts
import qs.Commons
import qs.Ui as Ui

FocusScope {
    id: root
    property var entries: []
    property bool busy: false
    property string error: ""
    property string workspaceError: ""
    signal workspaceRequested(string id, string workspace)
    signal configureRequested(string id)
    signal closeRequested()
    signal refreshRequested()
    signal arrangeRequested()
    signal toggleRequested(string id, bool enabled)
    focus: true
    Keys.onEscapePressed: closeRequested()
    Rectangle { anchors.fill: parent; color: Color.background; radius: Style.cornerRadius; border.width: 1; border.color: Color.muted }
    ColumnLayout {
        anchors.fill: parent; anchors.margins: Style.space(20); spacing: Style.space(14)
        RowLayout {
            Layout.fillWidth: true
            Label { text: "Desktop widgets"; font.pixelSize: Style.font.heading; font.bold: true; Layout.fillWidth: true }
            Ui.Button { text: "Close"; focusable: true; onClicked: root.closeRequested() }
        }
        Label { text: "Installed widgets"; color: Color.muted; Layout.fillWidth: true }
        ListView {
            Layout.fillWidth: true; Layout.fillHeight: true; clip: true; spacing: Style.space(8)
            model: root.entries
            delegate: RowLayout {
                required property var modelData
                width: ListView.view.width; height: Style.space(124)
                ColumnLayout {
                    Layout.fillWidth: true
                    Label { text: modelData.manifest.name; font.bold: true; Layout.fillWidth: true }
                    Label { text: modelData.manifest.version + " · " + modelData.manifest.id; color: Color.muted; font.pixelSize: Style.font.bodySmall; Layout.fillWidth: true }
                    Label { text: "Unplaced — no available cells or desktop geometry"; visible:!!modelData.placement && modelData.placement.enabled && !modelData.effective; color:Color.urgent; font.pixelSize:Style.font.bodySmall; Layout.fillWidth:true }
                    RowLayout {
                        visible:!!modelData.placement
                        Label { text:"Workspace"; color:Color.muted; font.pixelSize:Style.font.bodySmall }
                        Rectangle {
                            Layout.preferredWidth:Style.space(64); Layout.preferredHeight:Style.space(28)
                            color:Color.background; border.color:workspace.activeFocus?Color.accent:Color.muted; border.width:1
                            TextInput {
                                id:workspace;objectName:"workspace-input"
                                anchors.fill:parent;anchors.margins:Style.space(4)
                                text:modelData.placement && modelData.placement.workspace ? String(modelData.placement.workspace) : "all"
                                color:Color.foreground;font.family:Style.font.family;font.pixelSize:Style.font.bodySmall
                                selectByMouse:true;maximumLength:4
                                validator:RegularExpressionValidator { regularExpression: /all|[1-9][0-9]{0,3}/ }
                                Accessible.name:"Workspace for "+modelData.manifest.name+"; all or a number"
                                onAccepted:if(acceptableInput&&!root.busy)root.workspaceRequested(modelData.instanceId,text)
                            }
                        }
                        Ui.Button { text:"Set"; focusable:true;enabled:workspace.acceptableInput&&!root.busy;onClicked:root.workspaceRequested(modelData.instanceId,workspace.text) }
                    }
                }
                Ui.Button { text:"Settings"; visible:!!modelData.manifest.settingsEntryPoint && !!modelData.placement; focusable:true; onClicked:root.configureRequested(modelData.instanceId) }
                Ui.Button { text: modelData.placement && modelData.placement.enabled ? "Hide" : "Add"; enabled: !root.busy; focusable: true; onClicked: root.toggleRequested(modelData.instanceId || modelData.manifest.id, !(modelData.placement && modelData.placement.enabled)) }
            }
            Label { anchors.centerIn: parent; visible: !root.entries.length; text: "No widgets installed yet.\nInstall a widget package to get started."; horizontalAlignment: Text.AlignHCenter }
        }
        Label { text:root.workspaceError;visible:text!=="";color:Color.urgent;wrapMode:Text.Wrap;Layout.fillWidth:true }
        Label { text: root.error; visible: text !== ""; color: Color.urgent; wrapMode: Text.Wrap; Layout.fillWidth: true }
        RowLayout {
            Layout.fillWidth: true
            Ui.Button { text: root.busy ? "Working…" : "Refresh"; enabled: !root.busy; focusable: true; onClicked: root.refreshRequested() }
            Item { Layout.fillWidth: true }
            Ui.Button { text: "Arrange widgets"; focusable: true; onClicked: root.arrangeRequested() }
        }
    }
}
