import QtQuick
import QtQuick.Layouts
import QtQuick.Dialogs
import qs.Commons
import qs.Ui as Ui

FocusScope {
    id: root
    property var entries: []
    property bool busy: false
    property string error: ""
    property string workspaceError: ""
    property string search: ""
    property string selectedId: ""
    property string selectedSize: ""
    property string installPath: ""
    property bool installOpen: false
    readonly property var selectedEntry: entryFor(selectedId)
    readonly property var packages: packageEntries()
    readonly property var visiblePackages: packages.filter(function(item) {
        return !search || item.manifest.name.toLowerCase().indexOf(search.toLowerCase()) >= 0;
    })
    signal workspaceRequested(string id, string workspace)
    signal configureRequested(string id)
    signal closeRequested()
    signal refreshRequested()
    signal arrangeRequested()
    signal toggleRequested(string id, bool enabled)
    signal addRequested(string id, string size)
    signal installRequested(string path)
    focus: true
    Keys.onEscapePressed: closeRequested()

    function packageEntries() {
        var seen = {};
        return entries.filter(function(item) {
            var id = item.manifest.id;
            if (seen[id]) return false;
            seen[id] = true;
            return true;
        });
    }
    function entryFor(id) {
        for (var i = 0; i < entries.length; i++)
            if (entries[i].instanceId === id) return entries[i];
        return null;
    }
    function countFor(id) {
        var count = 0;
        for (var i = 0; i < entries.length; i++)
            if (entries[i].manifest.id === id && entries[i].placement && entries[i].placement.enabled) count++;
        return count;
    }
    onEntriesChanged: {
        if (!entryFor(selectedId)) selectedId = entries.length ? entries[0].instanceId : "";
    }
    onSelectedIdChanged: selectedSize = selectedEntry ? (selectedEntry.manifest.defaultFamily || "medium") : ""
    FolderDialog {
        id: packageFolder
        title: "Choose a widget package folder"
        onAccepted: {
            root.installPath = decodeURIComponent(selectedFolder.toString().replace(/^file:\/\//, ""));
            root.installOpen = true;
        }
    }

    Rectangle {
        anchors.fill: parent
        radius: Style.cornerRadius
        color: Color.background
        border.color: Qt.rgba(Color.foreground.r, Color.foreground.g, Color.foreground.b, 0.16)
        border.width: 1
    }
    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Style.space(16)
        spacing: Style.space(12)
        RowLayout {
            Layout.fillWidth: true
            Label { text: "Desktop widgets"; font.pixelSize: Style.font.heading; font.bold: true; Layout.fillWidth: true }
            Ui.Button { text: "Done"; onClicked: root.closeRequested() }
        }
        Label { text: "Choose a widget, then add it to your desktop."; color: Color.muted; Layout.fillWidth: true }
        Rectangle { Layout.fillWidth: true; height: 1; color: Qt.rgba(Color.foreground.r, Color.foreground.g, Color.foreground.b, 0.12) }
        RowLayout {
            Layout.fillWidth: true; Layout.fillHeight: true; spacing: Style.space(16)
            ColumnLayout {
                Layout.preferredWidth: Style.space(180); Layout.fillHeight: true; spacing: Style.space(8)
                Rectangle {
                    Layout.fillWidth: true; Layout.preferredHeight: Style.space(36)
                    radius: Style.space(8)
                    color: Qt.rgba(Color.foreground.r, Color.foreground.g, Color.foreground.b, 0.06)
                    border.color: searchInput.activeFocus ? Color.accent : "transparent"
                    border.width: 1
                    TextInput {
                        id: searchInput; objectName: "widget-search"
                        anchors.fill: parent; anchors.margins: Style.space(9)
                        color: Color.foreground; font.family: Style.font.family; font.pixelSize: Style.font.body
                        selectByMouse: true; activeFocusOnTab: true; clip: true
                        Accessible.name: "Search widgets"
                        onTextChanged: root.search = text
                    }
                    Label { anchors.verticalCenter: parent.verticalCenter; anchors.left: parent.left; anchors.leftMargin: Style.space(9); text: "Search widgets"; color: Color.muted; visible: !searchInput.text && !searchInput.activeFocus }
                }
                Label { text: "INSTALLED"; color: Color.muted; font.pixelSize: Style.font.bodySmall; Layout.topMargin: Style.space(8) }
                Ui.Button { text: "Install widget…"; Layout.fillWidth: true; onClicked: { root.installOpen = true; packageFolder.open(); } }
                ColumnLayout {
                    Layout.fillWidth: true; visible: root.installOpen; spacing: Style.space(6)
                    Label { text: "Package folder"; color: Color.muted; font.pixelSize: Style.font.bodySmall }
                    Rectangle { Layout.fillWidth: true; Layout.preferredHeight: Style.space(34); radius: Style.space(7); color: Qt.rgba(Color.foreground.r, Color.foreground.g, Color.foreground.b, 0.06)
                        TextInput { anchors.fill: parent; anchors.margins: Style.space(7); text: root.installPath; onTextEdited: root.installPath = text; activeFocusOnTab: true; selectByMouse: true; clip: true; color: Color.foreground; font.family: Style.font.family; font.pixelSize: Style.font.bodySmall; Accessible.name: "Widget package folder path" }
                    }
                    Ui.Button { text: "Install package"; enabled: !root.busy && root.installPath.length > 0; onClicked: root.installRequested(root.installPath) }
                }
                ListView {
                    id: categories
                    Layout.fillWidth: true; Layout.fillHeight: true; clip: true; spacing: Style.space(3)
                    model: root.visiblePackages
                    delegate: Rectangle {
                        required property var modelData
                        width: categories.width; height: Style.space(42)
                        radius: Style.space(8)
                        color: root.selectedEntry && root.selectedEntry.manifest.id === modelData.manifest.id
                            ? Qt.rgba(Color.accent.r, Color.accent.g, Color.accent.b, 0.18) : "transparent"
                        border.width: activeFocus ? 1 : 0; border.color: Color.accent
                        activeFocusOnTab: true
                        Accessible.role: Accessible.Button
                        Accessible.name: modelData.manifest.name
                        Accessible.onPressAction: root.selectedId = modelData.instanceId
                        Keys.onReturnPressed: root.selectedId = modelData.instanceId
                        Keys.onSpacePressed: root.selectedId = modelData.instanceId
                        RowLayout {
                            anchors.fill: parent; anchors.leftMargin: Style.space(10); anchors.rightMargin: Style.space(8)
                            Rectangle { width: Style.space(25); height: width; radius: Style.space(7); color: Qt.rgba(Color.accent.r, Color.accent.g, Color.accent.b, 0.22)
                                Label { anchors.centerIn: parent; text: modelData.manifest.name.charAt(0).toUpperCase(); font.bold: true; color: Color.accent }
                            }
                            Label { text: modelData.manifest.name; elide: Text.ElideRight; Layout.fillWidth: true }
                            Label { text: String(root.countFor(modelData.manifest.id)); color: Color.muted; visible: root.countFor(modelData.manifest.id) > 0 }
                        }
                        MouseArea { anchors.fill: parent; onClicked: { root.selectedId = modelData.instanceId; parent.forceActiveFocus(); } }
                    }
                }
            }
            Rectangle { Layout.fillHeight: true; width: 1; color: Qt.rgba(Color.foreground.r, Color.foreground.g, Color.foreground.b, 0.12) }
            ColumnLayout {
                Layout.fillWidth: true; Layout.fillHeight: true; spacing: Style.space(12)
                Label { text: root.selectedEntry ? root.selectedEntry.manifest.name : (root.search ? "No matching widgets" : "No widgets installed"); font.bold: true; font.pixelSize: Style.font.title; Layout.fillWidth: true }
                Label { text: root.selectedEntry ? "See available sizes, add an instance, or manage one already on your desktop." : (root.search ? "Try a different search." : "Install a widget package to get started."); color: Color.muted; wrapMode: Text.Wrap; Layout.fillWidth: true }
                Flickable {
                    Layout.fillWidth: true; Layout.fillHeight: true; clip: true
                    contentWidth: width; contentHeight: galleryContent.implicitHeight
                    ColumnLayout {
                        id: galleryContent; width: parent.width; spacing: Style.space(12)
                        Label { text: "SIZES"; color: Color.muted; font.pixelSize: Style.font.bodySmall; visible: !!root.selectedEntry }
                        Flow {
                            Layout.fillWidth: true; spacing: Style.space(10)
                            Repeater {
                                model: root.selectedEntry ? (root.selectedEntry.manifest.families || [root.selectedEntry.manifest.defaultFamily || "medium"]) : []
                                delegate: Rectangle {
                                    required property string modelData
                                    readonly property bool small: modelData === "small"
                                    readonly property bool large: modelData === "large"
                                    width: small ? Style.space(112) : Style.space(180)
                                    height: large ? Style.space(148) : Style.space(112)
                                    radius: Style.space(12)
                                    color: Qt.rgba(Color.accent.r, Color.accent.g, Color.accent.b, 0.13)
                                    border.color: root.selectedSize === modelData ? Color.accent : Qt.rgba(Color.accent.r, Color.accent.g, Color.accent.b, 0.38)
                                    border.width: root.selectedSize === modelData ? 2 : 1
                                    activeFocusOnTab: true
                                    Accessible.role: Accessible.Button
                                    Accessible.name: "Select " + modelData + " size"
                                    Accessible.onPressAction: root.selectedSize = modelData
                                    Keys.onReturnPressed: root.selectedSize = modelData
                                    Keys.onSpacePressed: root.selectedSize = modelData
                                    MouseArea { anchors.fill: parent; onClicked: { root.selectedSize = modelData; parent.forceActiveFocus(); } }
                                    Column {
                                        anchors.centerIn: parent; spacing: Style.space(7)
                                        Label { anchors.horizontalCenter: parent.horizontalCenter; text: root.selectedEntry ? root.selectedEntry.manifest.name : ""; font.bold: true; font.pixelSize: Style.font.bodySmall }
                                        Label { anchors.horizontalCenter: parent.horizontalCenter; text: modelData.charAt(0).toUpperCase() + modelData.slice(1); color: Color.muted; font.pixelSize: Style.font.bodySmall }
                                    }
                                }
                            }
                        }
                        Ui.Button { text: "Add to desktop"; enabled: !root.busy && !!root.selectedSize; visible: !!root.selectedEntry; onClicked: root.addRequested(root.selectedEntry.manifest.id, root.selectedSize) }
                        Label { text: "Content previews are coming; these cards show each available size."; color: Color.muted; font.pixelSize: Style.font.bodySmall; wrapMode: Text.Wrap; Layout.fillWidth: true; visible: !!root.selectedEntry }
                        Label { text: "ON YOUR DESKTOP"; color: Color.muted; font.pixelSize: Style.font.bodySmall; Layout.topMargin: Style.space(9); visible: !!root.selectedEntry }
                        Repeater {
                            model: root.selectedEntry ? root.entries.filter(function(item) { return item.manifest.id === root.selectedEntry.manifest.id && item.placement && item.placement.enabled; }) : []
                            delegate: RowLayout {
                                required property var modelData
                                Layout.fillWidth: true; spacing: Style.space(8)
                                Label { text: (modelData.placement.size || "medium") + " · " + (modelData.placement.workspace && modelData.placement.workspace !== "all" ? "Workspace " + modelData.placement.workspace : "All workspaces"); Layout.fillWidth: true; elide: Text.ElideRight }
                                TextInput {
                                    id: workspace; objectName: "workspace-input"
                                    Layout.preferredWidth: Style.space(44)
                                    text: modelData.placement.workspace ? String(modelData.placement.workspace) : "all"
                                    color: Color.foreground; font.family: Style.font.family; font.pixelSize: Style.font.bodySmall
                                    activeFocusOnTab: true; selectByMouse: true; maximumLength: 4
                                    validator: RegularExpressionValidator { regularExpression: /all|[1-9][0-9]{0,3}/ }
                                    Accessible.name: "Workspace for " + modelData.manifest.name + "; all or a number"
                                    onAccepted: if (acceptableInput && !root.busy) root.workspaceRequested(modelData.instanceId, text)
                                }
                                Ui.Button { text: "Set"; enabled: workspace.acceptableInput && !root.busy; onClicked: root.workspaceRequested(modelData.instanceId, workspace.text) }
                                Ui.Button { text: "Settings"; visible: !!modelData.manifest.settingsEntryPoint; onClicked: root.configureRequested(modelData.instanceId) }
                                Ui.Button { text: "Remove from desktop"; enabled: !root.busy; onClicked: root.toggleRequested(modelData.instanceId, false) }
                            }
                        }
                        Label { text: "No room at the current placement. Arrange widgets to choose another spot."; visible: !!root.selectedEntry && !!root.selectedEntry.placement && root.selectedEntry.placement.enabled && !root.selectedEntry.effective; color: Color.urgent; wrapMode: Text.Wrap; Layout.fillWidth: true }
                    }
                }
            }
        }
        Label { text: root.workspaceError; visible: text !== ""; color: Color.urgent; wrapMode: Text.Wrap; Layout.fillWidth: true }
        Label { text: root.error; visible: text !== ""; color: Color.urgent; wrapMode: Text.Wrap; Layout.fillWidth: true }
        Rectangle { Layout.fillWidth: true; height: 1; color: Qt.rgba(Color.foreground.r, Color.foreground.g, Color.foreground.b, 0.12) }
        RowLayout {
            Layout.fillWidth: true
            Ui.Button { text: root.busy ? "Working…" : "Refresh"; enabled: !root.busy; onClicked: root.refreshRequested() }
            Item { Layout.fillWidth: true }
            Ui.Button { text: "Arrange widgets"; onClicked: root.arrangeRequested() }
        }
    }
}
