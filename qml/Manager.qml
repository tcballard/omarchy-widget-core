import QtQuick
import QtQuick.Layouts
import QtQuick.Dialogs
import qs.Commons

FocusScope {
    id: root
    property var entries: []
    property var catalog: []
    property var retained: []
    property var monitors: []
    property string notice: ""
    property var pendingEditor: null
    property bool repairRequired: false
    signal cancelEditorRequested(string instanceId,string serial)
    signal repairRequested()
    property bool busy: false
    property string error: ""
    property string workspaceError: ""
    property string view: "instances"
    property string search: ""
    property string installPath: ""
    property bool installOpen: false
    readonly property var filteredCatalog: catalog.filter(function(e) {
        var query=root.search.trim().toLowerCase();
        return !query || (e.manifest.name+" "+e.packageId).toLowerCase().indexOf(query)>=0;
    })
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
    signal installRequested(string path)
    signal duplicateRequested(string id)
    signal removeRequested(string id)
    signal uninstallRequested(string packageId, string policy)
    signal weatherPermissionRequested(string packageId, bool allowed)
    signal packageControlRequested(string packageId, string action)
    signal rollbackRequested(string packageId)
    signal exportRequested(string packageId)
    signal recoverRequested(string instanceId, string family, string monitor)
    property var confirmationOrigin: null
    function ask(kind,id,name) {
        if(!busy) {
            confirmationOrigin=root.Window.window ? root.Window.window.activeFocusItem : null;
            confirmation={kind:kind,id:id,name:name};
            Qt.callLater(function() { confirmationCancel.forceActiveFocus(); });
        }
    }
    function dismissConfirmation() {
        confirmation=null;
        if(confirmationOrigin && confirmationOrigin.visible && confirmationOrigin.enabled) confirmationOrigin.forceActiveFocus();
        else root.forceActiveFocus();
        confirmationOrigin=null;
    }
    function confirm(policy) {
        if(busy || !confirmation)return;
        var action=confirmation; dismissConfirmation();
        if(action.kind === "instance") removeRequested(action.id);
        else uninstallRequested(action.id,policy);
    }
    function previewUrl(entry,family) {
        var path=entry.manifest.previews && entry.manifest.previews[family];
        return path ? "file://" + entry.directory.split("/").concat(path.split("/")).map(encodeURIComponent).join("/") : "";
    }
    FolderDialog {
        id:packageFolder
        title:"Choose a widget package folder"
        onAccepted: {
            root.installPath=decodeURIComponent(selectedFolder.toString().replace(/^file:\/\//,""));
            root.installOpen=true;
        }
    }

    property var instanceDetails: ({})
    property var packageDetails: ({})
    readonly property color hairline: Qt.rgba(Color.foreground.r, Color.foreground.g, Color.foreground.b, 0.12)
    readonly property color surface: Qt.rgba(Color.foreground.r, Color.foreground.g, Color.foreground.b, 0.025)
    function toggleDetails(kind, id) {
        var values=Object.assign({}, kind === "instance" ? instanceDetails : packageDetails);
        values[id]=!values[id];
        if(kind === "instance") instanceDetails=values; else packageDetails=values;
    }
    function titleCase(value) { return value.charAt(0).toUpperCase()+value.slice(1); }
    function instanceName(id) {
        for(var i=0;i<instances.length;i++) if(instances[i].instanceId===id) return instances[i].manifest.name;
        return "widget";
    }
    focus: true
    Keys.onEscapePressed: {
        if(confirmation) dismissConfirmation();
        else closeRequested();
    }
    Rectangle { anchors.fill: parent; color: Color.background; radius: Style.cornerRadius; border.width: 1; border.color: root.hairline }
    ColumnLayout {
        anchors.fill: parent; anchors.margins: Style.space(24); spacing: Style.space(18)
        enabled: !root.confirmation
        RowLayout {
            Layout.fillWidth: true; spacing: Style.space(12)
            ColumnLayout {
                Layout.fillWidth:true; spacing: Style.space(5)
                Label { text:"Widgets"; font.pixelSize:Style.space(26); font.weight:Font.DemiBold }
                Label { text:"Choose what appears on your desktop."; color:Color.muted; font.pixelSize:Style.space(13); Layout.fillWidth:true; wrapMode:Text.Wrap }
            }
            ManagerButton { text:"Arrange"; visible:root.instances.length>0; enabled:!root.busy; onClicked:root.arrangeRequested() }
            ManagerButton { text:"Close"; tone:"quiet"; onClicked:root.closeRequested() }
        }
        RowLayout {
            Layout.fillWidth:true; spacing:Style.space(4)
            ManagerButton { objectName:"instances-tab"; text:"Your widgets  ·  "+root.instances.length; selected:root.view==="instances"; onClicked:root.view="instances" }
            ManagerButton { objectName:"available-tab"; text:"Add widgets  ·  "+root.catalog.length; selected:root.view==="available"; onClicked:root.view="available" }
            Item { Layout.fillWidth:true }
        }
        RowLayout {
            visible:root.pendingEditor!==null; Layout.fillWidth:true
            Label { text:root.pendingEditor ? "Settings are open for "+root.instanceName(root.pendingEditor.instance)+"." : ""; Layout.fillWidth:true; wrapMode:Text.Wrap; font.pixelSize:Style.space(13) }
            ManagerButton { text:"Cancel settings"; enabled:!root.busy; onClicked:root.cancelEditorRequested(root.pendingEditor.instance,String(root.pendingEditor.serial)) }
        }
        ManagerButton { text:"Repair saved layout"; visible:root.repairRequired; enabled:!root.busy; onClicked:root.repairRequested() }
        RowLayout {
            visible:root.view==="available"; Layout.fillWidth:true; spacing:Style.space(8)
            Rectangle {
                Layout.fillWidth:true; Layout.preferredHeight:Style.space(38)
                color:root.surface; radius:Style.space(8); border.width:1; border.color:root.hairline
                TextInput {
                    id:searchInput; objectName:"widget-search"
                    anchors.fill:parent; anchors.margins:Style.space(9)
                    color:Color.foreground; font.family:Style.font.family; font.pixelSize:Style.font.body
                    selectByMouse:true; activeFocusOnTab:true; clip:true
                    Accessible.name:"Search widgets"
                    onTextChanged:root.search=text
                }
                Label { anchors.verticalCenter:parent.verticalCenter; anchors.left:parent.left; anchors.leftMargin:Style.space(9); text:"Search widgets"; color:Color.muted; visible:!searchInput.text && !searchInput.activeFocus }
            }
            ManagerButton { text:"Install widget…"; enabled:!root.busy; onClicked: { root.installOpen=true; packageFolder.open(); } }
        }
        RowLayout {
            visible:root.view==="available" && root.installOpen; Layout.fillWidth:true; spacing:Style.space(8)
            Label { text:"Package folder"; color:Color.muted }
            TextInput {
                Layout.fillWidth:true; text:root.installPath; onTextEdited:root.installPath=text
                color:Color.foreground; font.family:Style.font.family; selectByMouse:true; activeFocusOnTab:true
                Accessible.name:"Widget package folder path"
            }
            ManagerButton { text:"Install package"; enabled:!root.busy && root.installPath.length>0; onClicked:root.installRequested(root.installPath) }
        }
        ListView {
            id: available
            objectName:"available-list"
            visible:root.view==="available"
            Layout.fillWidth:true; Layout.fillHeight:true; clip:true; spacing:Style.space(12)
            cacheBuffer:100000
            function ensureVisible(item) { var p=item.mapToItem(contentItem,0,0); if(p.y<contentY)contentY=p.y;else if(p.y+item.height>contentY+height)contentY=p.y+item.height-height; }
            model:root.filteredCatalog
            delegate: Rectangle {
                id:packageCard
                required property var modelData
                property string family:modelData.manifest.defaultFamily || "small"
                readonly property bool expanded:!!root.packageDetails[modelData.packageId]
                readonly property bool needsAttention:!!modelData.problem || !!modelData.loadFailure || !!modelData.packageDisabled || (!!modelData.health && ["failed","recovery-blocked"].indexOf(modelData.health.state)>=0)
                width:ListView.view.width; height:packageContent.implicitHeight+Style.space(36)
                color:root.surface; border.width:1; border.color:root.hairline; radius:Math.min(Style.cornerRadius,Style.space(12))
                ColumnLayout {
                    id:packageContent
                    anchors.left:parent.left; anchors.right:parent.right; anchors.top:parent.top; anchors.margins:Style.space(18); spacing:Style.space(14)
                    RowLayout {
                        Layout.fillWidth:true; spacing:Style.space(20)
                        Rectangle {
                            Layout.preferredWidth:Style.space(112); Layout.preferredHeight:Style.space(128)
                            radius:Style.space(8); color:Qt.rgba(Color.accent.r,Color.accent.g,Color.accent.b,0.06)
                            FamilyPreview { anchors.fill:parent; anchors.margins:Style.space(8); family:packageCard.family; imageSource:root.previewUrl(packageCard.modelData,packageCard.family) }
                        }
                        ColumnLayout {
                            Layout.fillWidth:true; spacing:Style.space(10)
                            Label { text:packageCard.modelData.manifest.name; font.pixelSize:Style.space(17); font.weight:Font.DemiBold; Layout.fillWidth:true; wrapMode:Text.Wrap }
                            Label { text:packageCard.modelData.instanceCount ? packageCard.modelData.instanceCount+" on your widget list" : "Ready to add to your desktop"; color:Color.muted; font.pixelSize:Style.space(12); Layout.fillWidth:true; wrapMode:Text.Wrap }
                            Flow {
                                Layout.fillWidth:true; spacing:Style.space(4)
                                Repeater {
                                    model:packageCard.modelData.manifest.families || []
                                    ManagerButton { required property string modelData; objectName:"family-"+packageCard.modelData.packageId+"-"+modelData; text:root.titleCase(modelData); selected:packageCard.family===modelData; onClicked:packageCard.family=modelData }
                                }
                            }
                            ManagerButton { objectName:"add-"+packageCard.modelData.packageId; text:"Add widget"; tone:"primary"; enabled:!root.busy && !packageCard.modelData.problem; onClicked:root.createRequested(packageCard.modelData.packageId,packageCard.family) }
                        }
                    }
                    Label { text:packageCard.modelData.problem || (packageCard.modelData.loadFailure ? "Couldn't load this widget. Your settings are safe. Open Manage package to recover it." : packageCard.modelData.packageDisabled ? "This package is disabled. Enable it in Manage package." : packageCard.modelData.health && packageCard.needsAttention ? packageCard.modelData.health.message || "This widget needs attention." : ""); visible:text!==""; color:Color.urgent; Layout.fillWidth:true; wrapMode:Text.Wrap; font.pixelSize:Style.space(13) }
                    RowLayout {
                        Layout.fillWidth:true
                        Label { text:root.previewUrl(packageCard.modelData,packageCard.family) ? "Widget preview" : "Size preview"; color:Color.muted; font.pixelSize:Style.space(11); Layout.fillWidth:true }
                        ManagerButton { objectName:"package-details-"+packageCard.modelData.packageId; text:packageCard.expanded ? "Less  −" : "Manage package  +"; tone:"quiet"; onClicked:root.toggleDetails("package",packageCard.modelData.packageId) }
                    }
                    ColumnLayout {
                        visible:packageCard.expanded; Layout.fillWidth:true; spacing:Style.space(12)
                        Rectangle { Layout.fillWidth:true; implicitHeight:1; color:root.hairline }
                        Label { text:packageCard.modelData.packageId+"  ·  v"+packageCard.modelData.manifest.version; color:Color.muted; font.pixelSize:Style.space(11); Layout.fillWidth:true; elide:Text.ElideMiddle }
                        Label { text:packageCard.modelData.health ? "Status: "+packageCard.modelData.health.state+(packageCard.modelData.health.message ? " · "+packageCard.modelData.health.message : "") : "Status unavailable"; color:Color.muted; Layout.fillWidth:true; wrapMode:Text.Wrap; font.pixelSize:Style.space(12) }
                        Flow {
                            Layout.fillWidth:true; spacing:Style.space(6)
                            ManagerButton { text:"Restart"; enabled:!root.busy; onClicked:root.packageControlRequested(packageCard.modelData.packageId,"restart") }
                            ManagerButton { text:packageCard.modelData.packageDisabled ? "Enable package" : "Disable package"; enabled:!root.busy; onClicked:root.packageControlRequested(packageCard.modelData.packageId,packageCard.modelData.packageDisabled ? "enable" : "disable") }
                            ManagerButton { text:"Export settings"; enabled:!root.busy; onClicked:root.exportRequested(packageCard.modelData.packageId) }
                            ManagerButton { text:"Roll back update"; enabled:!root.busy; onClicked:root.rollbackRequested(packageCard.modelData.packageId) }
                            ManagerButton { objectName:"uninstall-"+packageCard.modelData.packageId; text:"Uninstall…"; tone:"danger"; enabled:!root.busy; onClicked:root.ask("package",packageCard.modelData.packageId,packageCard.modelData.manifest.name) }
                        }
                        Label { visible:(packageCard.modelData.manifest.capabilities || []).indexOf("weather")>=0; text:"Weather shares this widget's coordinates with Open-Meteo. Access applies to this installed version."; color:Color.muted; Layout.fillWidth:true; wrapMode:Text.Wrap; font.pixelSize:Style.space(12) }
                        ManagerButton { visible:(packageCard.modelData.manifest.capabilities || []).indexOf("weather")>=0; text:packageCard.modelData.weatherAllowed ? "Revoke weather access" : "Allow weather access"; enabled:!root.busy; onClicked:root.weatherPermissionRequested(packageCard.modelData.packageId,!packageCard.modelData.weatherAllowed) }
                    }
                }
            }
            Label { anchors.centerIn:parent; width:parent.width-Style.space(48); visible:!root.filteredCatalog.length; text:root.search ? "No matching widgets" : "No widgets available yet\nInstall a widget package to add it here."; font.pixelSize:Style.space(14); color:Color.muted; wrapMode:Text.Wrap; horizontalAlignment:Text.AlignHCenter }
        }
        ListView {
            id:instancesList
            objectName:"instances-list"
            visible:root.view==="instances"
            Layout.fillWidth:true; Layout.fillHeight:true; clip:true; spacing:Style.space(12)
            cacheBuffer:100000
            function ensureVisible(item) { var p=item.mapToItem(contentItem,0,0); if(p.y<contentY)contentY=p.y;else if(p.y+item.height>contentY+height)contentY=p.y+item.height-height; }
            model:root.instances
            delegate: Rectangle {
                id:instanceCard
                required property var modelData
                readonly property bool available:!modelData.uninstalled
                readonly property bool expanded:!!root.instanceDetails[modelData.instanceId]
                readonly property bool unplaced:available && modelData.placement.enabled && !modelData.effective
                property string chosenSize:modelData.placement.size || "medium"
                property string chosenMonitor:""
                width:ListView.view.width; height:instanceContent.implicitHeight+Style.space(36)
                color:root.surface; border.width:1; border.color:root.hairline; radius:Math.min(Style.cornerRadius,Style.space(12))
                ColumnLayout {
                    id:instanceContent
                    anchors.left:parent.left; anchors.right:parent.right; anchors.top:parent.top; anchors.margins:Style.space(18); spacing:Style.space(14)
                    RowLayout {
                        Layout.fillWidth:true; spacing:Style.space(14)
                        Rectangle {
                            Layout.preferredWidth:Style.space(44); Layout.preferredHeight:Style.space(44); radius:Style.space(10)
                            color:Qt.rgba(Color.accent.r,Color.accent.g,Color.accent.b,0.10)
                            Grid {
                                anchors.centerIn:parent; columns:2; spacing:Style.space(3)
                                Repeater { model:4; Rectangle { width:Style.space(9); height:width; radius:Style.space(2); color:Color.accent; opacity:index < (instanceCard.modelData.placement.size==="large" ? 4 : instanceCard.modelData.placement.size==="medium" ? 2 : 1) ? 0.9 : 0.18 } }
                            }
                        }
                        ColumnLayout {
                            Layout.fillWidth:true; spacing:Style.space(5)
                            Label { text:instanceCard.modelData.manifest.name; font.pixelSize:Style.space(17); font.weight:Font.DemiBold; Layout.fillWidth:true; wrapMode:Text.Wrap }
                            Label { text:root.titleCase(instanceCard.modelData.placement.size || "medium")+"  ·  "+(instanceCard.modelData.placement.workspace ? "Workspace "+instanceCard.modelData.placement.workspace : "All workspaces"); color:Color.muted; font.pixelSize:Style.space(12); Layout.fillWidth:true; wrapMode:Text.Wrap }
                        }
                        Label { text:!instanceCard.available ? "Saved" : !instanceCard.modelData.placement.enabled ? "Hidden" : instanceCard.unplaced ? "Needs space" : "On desktop"; color:instanceCard.unplaced ? Color.urgent : Color.muted; font.pixelSize:Style.space(12) }
                    }
                    Label { visible:!instanceCard.available || instanceCard.unplaced; text:!instanceCard.available ? "This package was uninstalled. Your settings are kept for when you reinstall it." : "There's no free position for this widget. Choose a size and display below."; color:instanceCard.unplaced ? Color.urgent : Color.muted; Layout.fillWidth:true; wrapMode:Text.Wrap; font.pixelSize:Style.space(13) }
                    Flow {
                        visible:instanceCard.unplaced; Layout.fillWidth:true; spacing:Style.space(6)
                        Repeater { model:instanceCard.modelData.manifest.families || []; ManagerButton { required property string modelData; objectName:"resize-"+instanceCard.modelData.instanceId+"-"+modelData; text:root.titleCase(modelData); selected:instanceCard.chosenSize===modelData; onClicked:instanceCard.chosenSize=modelData } }
                        ManagerButton { text:"Automatic display"; selected:instanceCard.chosenMonitor===""; onClicked:instanceCard.chosenMonitor="" }
                        Repeater { model:root.monitors; ManagerButton { required property string modelData; text:modelData; selected:instanceCard.chosenMonitor===modelData; onClicked:instanceCard.chosenMonitor=modelData } }
                        ManagerButton { objectName:"recover-"+instanceCard.modelData.instanceId; text:"Find a position"; tone:"primary"; enabled:!root.busy; onClicked:root.recoverRequested(instanceCard.modelData.instanceId,instanceCard.chosenSize,instanceCard.chosenMonitor) }
                    }
                    Flow {
                        Layout.fillWidth:true; spacing:Style.space(6)
                        ManagerButton { objectName:"configure-"+instanceCard.modelData.instanceId; text:"Settings"; tone:"primary"; visible:instanceCard.available && (!!instanceCard.modelData.manifest.settingsEntryPoint || instanceCard.modelData.manifest.renderer==="declarative"); enabled:!root.busy; onClicked:root.configureRequested(instanceCard.modelData.instanceId) }
                        ManagerButton { objectName:"toggle-"+instanceCard.modelData.instanceId; text:instanceCard.modelData.placement.enabled ? "Hide" : "Show"; visible:instanceCard.available; enabled:!root.busy; onClicked:root.toggleRequested(instanceCard.modelData.instanceId,!instanceCard.modelData.placement.enabled) }
                        ManagerButton { objectName:"instance-details-"+instanceCard.modelData.instanceId; text:instanceCard.expanded ? "Less  −" : "More  +"; accessibleText:(instanceCard.expanded ? "Hide options for " : "More options for ")+instanceCard.modelData.manifest.name; tone:"quiet"; onClicked:root.toggleDetails("instance",instanceCard.modelData.instanceId) }
                    }
                    ColumnLayout {
                        visible:instanceCard.expanded; Layout.fillWidth:true; spacing:Style.space(12)
                        Rectangle { Layout.fillWidth:true; implicitHeight:1; color:root.hairline }
                        RowLayout {
                            visible:instanceCard.available; Layout.fillWidth:true; spacing:Style.space(8)
                            Label { text:"Workspace"; font.pixelSize:Style.space(13) }
                            Rectangle {
                                Layout.preferredWidth:Style.space(72); Layout.preferredHeight:Style.space(36); radius:Style.space(6)
                                color:Color.background; border.color:workspace.activeFocus ? Color.accent : root.hairline; border.width:1
                                TextInput {
                                    id:workspace; objectName:"workspace-input"
                                    anchors.fill:parent; anchors.margins:Style.space(8); verticalAlignment:TextInput.AlignVCenter
                                    text:instanceCard.modelData.placement.workspace ? String(instanceCard.modelData.placement.workspace) : "all"
                                    color:Color.foreground; font.family:Style.font.family; font.pixelSize:Style.space(13)
                                    selectByMouse:true; maximumLength:4
                                    validator:RegularExpressionValidator { regularExpression:/all|[1-9][0-9]{0,3}/ }
                                    Accessible.name:"Workspace for "+instanceCard.modelData.manifest.name+"; all or a number"
                                    onActiveFocusChanged:if(activeFocus)instancesList.ensureVisible(workspace)
                                    onAccepted:if(acceptableInput && !root.busy)root.workspaceRequested(instanceCard.modelData.instanceId,text)
                                }
                            }
                            ManagerButton { text:"Apply"; enabled:workspace.acceptableInput && !root.busy; onClicked:root.workspaceRequested(instanceCard.modelData.instanceId,workspace.text) }
                            Item { Layout.fillWidth:true }
                        }
                        Label { text:"Display: "+(instanceCard.modelData.effective ? instanceCard.modelData.effective.monitor : instanceCard.modelData.placement.monitor || "Automatic")+" · Enter all or a workspace number above."; color:Color.muted; font.pixelSize:Style.space(12); Layout.fillWidth:true; wrapMode:Text.Wrap }
                        Flow {
                            Layout.fillWidth:true; spacing:Style.space(6)
                            ManagerButton { objectName:"duplicate-"+instanceCard.modelData.instanceId; text:"Duplicate"; visible:instanceCard.available; enabled:!root.busy; onClicked:root.duplicateRequested(instanceCard.modelData.instanceId) }
                            ManagerButton { objectName:"remove-"+instanceCard.modelData.instanceId; text:"Remove…"; tone:"danger"; enabled:!root.busy; onClicked:root.ask("instance",instanceCard.modelData.instanceId,instanceCard.modelData.manifest.name) }
                        }
                        Label { text:instanceCard.modelData.instanceId; color:Color.muted; font.pixelSize:Style.space(10); Layout.fillWidth:true; elide:Text.ElideMiddle }
                    }
                }
            }
            ColumnLayout {
                anchors.centerIn:parent; width:parent.width-Style.space(48); spacing:Style.space(12); visible:!root.instances.length
                Label { text:"Your desktop, your widgets"; font.pixelSize:Style.space(18); font.weight:Font.DemiBold; Layout.alignment:Qt.AlignHCenter }
                Label { text:"Choose a widget and a size to get started."; color:Color.muted; font.pixelSize:Style.space(13); Layout.fillWidth:true; horizontalAlignment:Text.AlignHCenter; wrapMode:Text.Wrap }
                ManagerButton { text:"Browse widgets"; tone:"primary"; Layout.alignment:Qt.AlignHCenter; onClicked:root.view="available" }
            }
        }
        Label { text:root.workspaceError; visible:text!==""; color:Color.urgent; wrapMode:Text.Wrap; Layout.fillWidth:true }
        Label { text:root.notice; visible:text!==""; wrapMode:Text.Wrap; Layout.fillWidth:true }
        Label { text:root.error; visible:text!==""; color:Color.urgent; wrapMode:Text.Wrap; Layout.fillWidth:true }
        RowLayout {
            Layout.fillWidth:true
            Label { text:root.busy ? "Updating…" : root.view==="available" ? "Each new widget has its own settings." : "Each widget keeps its own settings."; color:Color.muted; font.pixelSize:Style.space(11); Layout.fillWidth:true; wrapMode:Text.Wrap }
            ManagerButton { text:"Refresh"; tone:"quiet"; enabled:!root.busy; onClicked:root.refreshRequested() }
        }
    }
    Rectangle {
        id:confirmationPanel
        anchors.fill:parent; anchors.margins:1; radius:Style.cornerRadius; color:Color.background
        visible:!!root.confirmation; focus:visible
        ColumnLayout {
            anchors.centerIn:parent; width:Math.min(parent.width-Style.space(64),Style.space(460)); spacing:Style.space(20)
            Label { text:root.confirmation ? (root.confirmation.kind==="instance" ? "Remove "+root.confirmation.name+"?" : "Uninstall "+root.confirmation.name+"?") : ""; font.pixelSize:Style.space(23); font.weight:Font.DemiBold; Layout.fillWidth:true; wrapMode:Text.Wrap }
            Label { text:root.confirmation && root.confirmation.kind==="instance" ? "This removes this widget and its saved settings. Your other widgets and the installed package stay available." : "All widgets from this package will stop. Keep their settings to use again after reinstalling, or delete them permanently."; font.pixelSize:Style.space(14); lineHeight:1.3; Layout.fillWidth:true; wrapMode:Text.Wrap }
            Flow {
                Layout.fillWidth:true; spacing:Style.space(8)
                ManagerButton { id:confirmationCancel; objectName:"cancel-removal"; text:"Cancel"; onClicked:root.dismissConfirmation() }
                ManagerButton { objectName:"confirm-remove"; text:"Remove widget"; tone:"danger"; visible:!!root.confirmation && root.confirmation.kind==="instance"; enabled:!root.busy; onClicked:root.confirm("") }
                ManagerButton { objectName:"uninstall-keep"; text:"Uninstall · keep settings"; tone:"primary"; visible:!!root.confirmation && root.confirmation.kind==="package"; enabled:!root.busy; onClicked:root.confirm("keep") }
                ManagerButton { objectName:"uninstall-delete"; text:"Delete settings too"; tone:"danger"; visible:!!root.confirmation && root.confirmation.kind==="package"; enabled:!root.busy; onClicked:root.confirm("delete") }
            }
        }
    }
}
