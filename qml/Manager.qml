import QtQuick
import QtQuick.Layouts
import qs.Commons
import qs.Ui as Ui

FocusScope {
    id: root
    property var entries: []
    property var catalog: []
    property var retained: []
    property bool busy: false
    property string error: ""
    property string workspaceError: ""
    property string view: "instances"
    property var confirmation: null
    readonly property var instances: entries.filter(function(e) { return !!e.placement; }).concat(retained.map(function(e) {
        return Object.assign({},e,{uninstalled:true,manifest:{id:e.packageId,name:e.name || e.packageId}});
    }))
    signal workspaceRequested(string id, string workspace)
    signal configureRequested(string id)
    signal closeRequested()
    signal refreshRequested()
    signal arrangeRequested()
    signal toggleRequested(string id, bool enabled)
    signal createRequested(string packageId, string family)
    signal duplicateRequested(string id)
    signal removeRequested(string id)
    signal uninstallRequested(string packageId, string policy)
    signal weatherPermissionRequested(string packageId, bool allowed)
    signal packageControlRequested(string packageId, string action)
    signal rollbackRequested(string packageId)
    function ask(kind,id,name) { if(!busy) { confirmation={kind:kind,id:id,name:name}; confirmationPanel.forceActiveFocus(); } }
    function confirm(policy) {
        if(busy || !confirmation)return;
        var action=confirmation; confirmation=null;
        if(action.kind === "instance") removeRequested(action.id);
        else uninstallRequested(action.id,policy);
    }
    function previewUrl(entry,family) {
        var path=entry.manifest.previews && entry.manifest.previews[family];
        return path ? "file://" + entry.directory.split("/").concat(path.split("/")).map(encodeURIComponent).join("/") : "";
    }
    focus: true
    Keys.onEscapePressed: {
        if(confirmation) confirmation=null;
        else closeRequested();
    }
    Rectangle { anchors.fill: parent; color: Color.background; radius: Style.cornerRadius; border.width: 1; border.color: Color.muted }
    ColumnLayout {
        anchors.fill: parent; anchors.margins: Style.space(20); spacing: Style.space(12)
        enabled: !root.confirmation
        RowLayout {
            Layout.fillWidth: true
            Label { text: "Widgets"; font.pixelSize: Style.font.heading; font.bold: true; Layout.fillWidth: true }
            Ui.Button { text: "Close"; onClicked: root.closeRequested() }
        }
        RowLayout {
            Layout.fillWidth: true
            Ui.Button { objectName: "instances-tab"; text: "Your widgets · " + root.instances.length; selected: root.view === "instances"; onClicked: root.view="instances" }
            Ui.Button { objectName: "available-tab"; text: "Available · " + root.catalog.length; selected: root.view === "available"; onClicked: root.view="available" }
            Item { Layout.fillWidth: true }
        }
        Label { text: root.view === "available" ? "Choose a size to add a new widget with its default settings." : "Each widget has its own settings and desktop position."; color: Color.muted; wrapMode: Text.Wrap; Layout.fillWidth: true }
        ListView {
            id: available
            objectName: "available-list"
            visible: root.view === "available"
            Layout.fillWidth: true; Layout.fillHeight: true; clip: true; spacing: Style.space(12)
            model: root.catalog
            delegate: Rectangle {
                id: packageCard
                required property var modelData
                property string family: modelData.manifest.defaultFamily || "small"
                width: ListView.view.width; height: packageContent.implicitHeight + Style.space(24)
                color: "transparent"; border.width: 1; border.color: Color.muted; radius: Style.cornerRadius
                ColumnLayout {
                    id: packageContent
                    anchors.left: parent.left; anchors.right: parent.right; anchors.top: parent.top; anchors.margins: Style.space(12); spacing: Style.space(8)
                    RowLayout {
                        Layout.fillWidth: true
                        Label { text: packageCard.modelData.manifest.name; font.bold: true; Layout.fillWidth: true }
                        Label { text: String(packageCard.modelData.instanceCount) + " added"; color: Color.muted }
                    }
                    Label { text: packageCard.modelData.packageId + " · " + packageCard.modelData.manifest.version; color: Color.muted; font.pixelSize: Style.font.bodySmall; Layout.fillWidth: true; elide: Text.ElideMiddle }
                    FamilyPreview { Layout.fillWidth: true; Layout.preferredHeight: Style.space(110); family: packageCard.family; imageSource: root.previewUrl(packageCard.modelData,packageCard.family); visible: !packageCard.modelData.problem }
                    Label { text: packageCard.modelData.problem || (!packageCard.modelData.manifest.previews || !packageCard.modelData.manifest.previews[packageCard.family] ? "Size preview · no widget image supplied" : "Widget preview"); color: packageCard.modelData.problem ? Color.urgent : Color.muted; Layout.fillWidth: true; wrapMode: Text.Wrap; font.pixelSize: Style.font.bodySmall }
                    Label {
                        visible: (packageCard.modelData.manifest.capabilities || []).indexOf("weather")>=0
                        text: "Weather sends this widget's coordinates to Open-Meteo. Permission applies to this installed version."
                        color: Color.muted; Layout.fillWidth:true; wrapMode:Text.Wrap
                    }
                    Label {
                        text: packageCard.modelData.health ? "Runner: "+packageCard.modelData.health.state+" · failures "+packageCard.modelData.health.failures : "Runner status unavailable"
                        color: packageCard.modelData.health && packageCard.modelData.health.state==="failed" ? Color.urgent : Color.muted
                        Layout.fillWidth:true; wrapMode:Text.Wrap
                    }
                    Flow {
                        Layout.fillWidth:true; spacing:Style.space(6)
                        Ui.Button {text:"Restart"; enabled:!root.busy; onClicked:root.packageControlRequested(packageCard.modelData.packageId,"restart")}
                        Ui.Button {text:packageCard.modelData.packageDisabled ? "Enable package" : "Disable package"; enabled:!root.busy; onClicked:root.packageControlRequested(packageCard.modelData.packageId,packageCard.modelData.packageDisabled ? "enable" : "disable")}
                        Ui.Button {text:"Roll back update"; enabled:!root.busy; onClicked:root.rollbackRequested(packageCard.modelData.packageId)}
                    }
                    Ui.Button {
                        visible: (packageCard.modelData.manifest.capabilities || []).indexOf("weather")>=0
                        text: packageCard.modelData.weatherAllowed ? "Revoke weather access" : "Allow weather access"
                        enabled: !root.busy
                        onClicked: root.weatherPermissionRequested(packageCard.modelData.packageId,!packageCard.modelData.weatherAllowed)
                    }
                    Flow {
                        Layout.fillWidth: true; spacing: Style.space(6)
                        Repeater {
                            model: packageCard.modelData.manifest.families || []
                            Ui.Button {
                                required property string modelData
                                objectName: "family-" + packageCard.modelData.packageId + "-" + modelData
                                text: modelData.charAt(0).toUpperCase()+modelData.slice(1)
                                selected: packageCard.family === modelData
                                onClicked: packageCard.family=modelData
                            }
                        }
                        Ui.Button { objectName: "add-"+packageCard.modelData.packageId; text: "Add widget"; enabled: !root.busy && !packageCard.modelData.problem; onClicked: root.createRequested(packageCard.modelData.packageId,packageCard.family) }
                        Ui.Button { objectName: "uninstall-"+packageCard.modelData.packageId; text: "Uninstall…"; enabled: !root.busy; onClicked: root.ask("package",packageCard.modelData.packageId,packageCard.modelData.manifest.name) }
                    }
                }
            }
            Label { anchors.centerIn: parent; width: parent.width; visible: !root.catalog.length; text: "No widget packages installed.\nInstall a package to make it available here."; wrapMode: Text.Wrap; horizontalAlignment: Text.AlignHCenter }
        }
        ListView {
            id: instancesList
            objectName: "instances-list"
            visible: root.view === "instances"
            Layout.fillWidth: true; Layout.fillHeight: true; clip: true; spacing: Style.space(12)
            model: root.instances
            delegate: Rectangle {
                id: instanceCard
                required property var modelData
                readonly property bool available: !modelData.uninstalled
                width: ListView.view.width; height: instanceContent.implicitHeight + Style.space(24)
                color: "transparent"; border.width: 1; border.color: Color.muted; radius: Style.cornerRadius
                ColumnLayout {
                    id: instanceContent
                    anchors.left: parent.left; anchors.right: parent.right; anchors.top: parent.top; anchors.margins: Style.space(12); spacing: Style.space(8)
                    Label { text: instanceCard.modelData.manifest.name; font.bold: true; Layout.fillWidth: true }
                    Label { text: instanceCard.modelData.instanceId; color: Color.muted; font.pixelSize: Style.font.bodySmall; Layout.fillWidth: true; elide: Text.ElideMiddle }
                    Label {
                        text: (instanceCard.modelData.placement.size || "medium") + " · " + (instanceCard.modelData.placement.monitor || "Automatic monitor") + " · workspace " + (instanceCard.modelData.placement.workspace || "all")
                        color: Color.muted; font.pixelSize: Style.font.bodySmall; Layout.fillWidth: true; wrapMode: Text.Wrap
                    }
                    Label {
                        text: !instanceCard.available ? "Package uninstalled · settings kept. Reinstall it to show this widget." : !instanceCard.modelData.placement.enabled ? "Hidden" : !instanceCard.modelData.effective ? "Unplaced — no available cells or desktop geometry" : "On desktop · " + instanceCard.modelData.effective.monitor
                        color: instanceCard.available && instanceCard.modelData.placement.enabled && !instanceCard.modelData.effective ? Color.urgent : Color.muted
                        font.pixelSize: Style.font.bodySmall; Layout.fillWidth: true; wrapMode: Text.Wrap
                    }
                    RowLayout {
                        visible: instanceCard.available
                        Label { text: "Workspace"; color: Color.muted; font.pixelSize: Style.font.bodySmall }
                        Rectangle {
                            Layout.preferredWidth: Style.space(64); Layout.preferredHeight: Style.space(28)
                            color: Color.background; border.color: workspace.activeFocus ? Color.accent : Color.muted; border.width: 1
                            TextInput {
                                id: workspace; objectName: "workspace-input"
                                anchors.fill: parent; anchors.margins: Style.space(4)
                                text: instanceCard.modelData.placement.workspace ? String(instanceCard.modelData.placement.workspace) : "all"
                                color: Color.foreground; font.family: Style.font.family; font.pixelSize: Style.font.bodySmall
                                selectByMouse: true; maximumLength: 4
                                validator: RegularExpressionValidator { regularExpression: /all|[1-9][0-9]{0,3}/ }
                                Accessible.name: "Workspace for " + instanceCard.modelData.manifest.name + "; all or a number"
                                onAccepted: if(acceptableInput && !root.busy) root.workspaceRequested(instanceCard.modelData.instanceId,text)
                            }
                        }
                        Ui.Button { text: "Set"; enabled: workspace.acceptableInput && !root.busy; onClicked: root.workspaceRequested(instanceCard.modelData.instanceId,workspace.text) }
                    }
                    Flow {
                        Layout.fillWidth: true; spacing: Style.space(6)
                        Ui.Button { objectName: "configure-"+instanceCard.modelData.instanceId; text: "Configure"; visible: instanceCard.available && !!instanceCard.modelData.manifest.settingsEntryPoint; enabled: !root.busy; onClicked: root.configureRequested(instanceCard.modelData.instanceId) }
                        Ui.Button { text: "Arrange"; visible: instanceCard.available; enabled: !root.busy; onClicked: root.arrangeRequested() }
                        Ui.Button { objectName: "duplicate-"+instanceCard.modelData.instanceId; text: "Duplicate"; visible: instanceCard.available; enabled: !root.busy; onClicked: root.duplicateRequested(instanceCard.modelData.instanceId) }
                        Ui.Button { objectName: "toggle-"+instanceCard.modelData.instanceId; text: instanceCard.modelData.placement.enabled ? "Hide" : "Show"; visible: instanceCard.available; enabled: !root.busy; onClicked: root.toggleRequested(instanceCard.modelData.instanceId,!instanceCard.modelData.placement.enabled) }
                        Ui.Button { objectName: "remove-"+instanceCard.modelData.instanceId; text: "Remove…"; enabled: !root.busy; onClicked: root.ask("instance",instanceCard.modelData.instanceId,instanceCard.modelData.manifest.name) }
                    }
                }
            }
            Label { anchors.centerIn: parent; width: parent.width; visible: !root.instances.length; text: "Your desktop has no widgets yet.\nChoose Available to add one."; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.Wrap }
        }
        Label { text: root.workspaceError; visible: text!==""; color: Color.urgent; wrapMode: Text.Wrap; Layout.fillWidth: true }
        Label { text: root.error; visible: text!==""; color: Color.urgent; wrapMode: Text.Wrap; Layout.fillWidth: true }
        RowLayout {
            Layout.fillWidth: true
            Ui.Button { text: root.busy ? "Working…" : "Refresh"; enabled: !root.busy; onClicked: root.refreshRequested() }
            Item { Layout.fillWidth: true }
            Ui.Button { text: "Arrange widgets"; enabled: !root.busy; onClicked: root.arrangeRequested() }
        }
    }
    Rectangle {
        id: confirmationPanel
        anchors.fill: parent; anchors.margins: 1; color: Color.background
        visible: !!root.confirmation
        focus: visible
        ColumnLayout {
            anchors.centerIn: parent; width: parent.width - Style.space(48); spacing: Style.space(16)
            Label { text: root.confirmation ? (root.confirmation.kind === "instance" ? "Remove this widget?" : "Uninstall " + root.confirmation.name + "?") : ""; font.pixelSize: Style.font.heading; font.bold: true; Layout.fillWidth: true; wrapMode: Text.Wrap }
            Label { text: root.confirmation && root.confirmation.kind === "instance" ? "This deletes only this widget's settings and position. Other instances and its installed package are kept." : "All widgets from this package will stop. Keep their settings and positions for a later reinstall, or delete them."; Layout.fillWidth: true; wrapMode: Text.Wrap }
            Flow {
                Layout.fillWidth: true; spacing: Style.space(8)
                Ui.Button { objectName: "cancel-removal"; text: "Cancel"; onClicked: root.confirmation=null }
                Ui.Button { objectName: "confirm-remove"; text: "Remove widget"; visible: !!root.confirmation && root.confirmation.kind === "instance"; enabled: !root.busy; onClicked: root.confirm("") }
                Ui.Button { objectName: "uninstall-keep"; text: "Uninstall · keep settings"; visible: !!root.confirmation && root.confirmation.kind === "package"; enabled: !root.busy; onClicked: root.confirm("keep") }
                Ui.Button { objectName: "uninstall-delete"; text: "Uninstall · delete settings"; visible: !!root.confirmation && root.confirmation.kind === "package"; enabled: !root.busy; onClicked: root.confirm("delete") }
            }
        }
    }
}
