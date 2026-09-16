import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import qs.Commons
import "qml" as Core
import "qml/Grid.js" as Grid
import "qml/Workspace.js" as Workspace

Item {
    id: root
    property var shell: null
    property var manifest: null
    property string omarchyPath: ""
    readonly property bool managerRole: Quickshell.env("OMARCHY_WIDGET_ROLE") === "manager"
    property string editSerial: ""
    property var installed: []
    property var catalog: []
    property var retained: []
    property var occupancy: []
    property var desktop: ({available:false,monitors:{}})
    property var themeAppearance: ({})
    property string error: ""
    property bool shown: true
    property bool editing: false
    property bool managerOpen: false
    property var saveStates: ({})
    property var placementErrors: ({})
    property string configuring: ""
    property int settingsGeneration: 0
    property string editorError: ""
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("bin/omarchy-widget").toString().replace(/^file:\/\//, ""))
    function entry(id) {
        for (var i=0;i<installed.length;i++) if(installed[i].instanceId === id) return installed[i];
        return null;
    }
    function execute(args, token) {
        var accepted=operation.enqueue(args,token);
        if(!accepted) error="Too many pending operations. Please retry.";
        return accepted;
    }
    function refresh() { return execute(["list"]); }
    function control(method) { return execute(["control",method]); }
    function save(id, value, revision) {
        if (saveStates[id] && saveStates[id].saving) return false;
        if(configuring===id) editorError="";
        var token={instance:id,generation:configuring===id?settingsGeneration:-1};
        if (!execute(["save",id,JSON.stringify({revision:revision,settings:value})],token)) {
            if(configuring===id) editorError=error;
            return false;
        }
        var states=Object.assign({},saveStates); states[id]={saving:true,error:"",saved:false}; saveStates=states;
        return true;
    }
    function closeSettings() {
        if(configuring && editSerial && entry(configuring)) execute(["edit-done",configuring,editSerial]);
        configuring="";
        settingsGeneration++;
        settingsLoader.source="";
        settingsContext.draftSettings=({});
        editorError="";
    }
    function configure(id) {
        if(managerRole) { execute(["edit",id]); return; }
        // Repeated gear presses must not reset an unsaved draft.
        if(configuring===id) return;
        if(configuring!=="") { editorError="Save or cancel this editor before configuring another widget."; return; }
        if(saveStates[id] && saveStates[id].saving) { error="This widget is still saving. Please retry after it finishes."; return; }
        var item=entry(id);
        if(!item || !item.placement || !item.manifest.settingsEntryPoint) { error="This widget has no available settings editor."; return; }
        settingsGeneration++;
        editorError="";
        var states=Object.assign({},saveStates); delete states[id]; saveStates=states;
        settingsContext.draftSettings=JSON.parse(JSON.stringify(item.placement.settings));
        settingsContext.revision=item.placement.revision;
        configuring=id;
        settingsLoader.setSource("file://"+item.directory+"/"+item.manifest.settingsEntryPoint,{settingsContext:settingsContext});
    }
    Core.RegistryQueue {
        id: operation
        helper: root.helper
        onCompleted: function(request,success,response,message) {
            root.error=message;
            if(request.token) {
                var id=request.token.instance;
                var states=Object.assign({},root.saveStates);
                states[id]={saving:false,error:message,saved:success}; root.saveStates=states;
                if(success && root.configuring===id && root.settingsGeneration===request.token.generation) root.closeSettings();
            }
            if(request.args[0]==="place" || request.args[0]==="workspace") {
                var placementErrors=Object.assign({},root.placementErrors);
                placementErrors[request.args[1]]=message; root.placementErrors=placementErrors;
            }
            if(!success) { if(request.args[0]!=="list")root.refresh(); return; }
            if(request.args[0] === "list") {
                if(response.api !== 2 || !Array.isArray(response.installed)) { root.error="Unsupported Core response"; return; }
                // Preserve delegate focus and in-progress manager edits during polling.
                if(JSON.stringify(root.installed)!==JSON.stringify(response.installed)) root.installed=response.installed;
                if(!root.managerRole && root.configuring!=="" && !root.entry(root.configuring)) root.closeSettings();
                if(JSON.stringify(root.catalog)!==JSON.stringify(response.catalog || [])) root.catalog=response.catalog || [];
                if(JSON.stringify(root.retained)!==JSON.stringify(response.retained || [])) root.retained=response.retained || [];
                root.occupancy=response.occupancy || [];
                var desktop=response.desktop || ({available:false,monitors:{}});
                if(JSON.stringify(root.desktop)!==JSON.stringify(desktop)) root.desktop=desktop;
                if(response.runtime) {
                    root.shown=response.runtime.shown !== false;
                    root.editing=response.runtime.editing === true;
                    root.managerOpen=root.managerRole && response.runtime.managerOpen === true;
                    var edit=response.runtime.edit;
                    if(!root.managerRole && root.configuring==="" && edit && String(edit.serial)!==root.editSerial) {
                        root.editSerial=String(edit.serial); root.configure(edit.instance);
                    }
                }
                root.themeAppearance=response.appearance || {};
                Color.apply(response.palette || {});
                Style.apply(root.themeAppearance);
                root.error=(response.problems || []).join("; ");
            } else {
                // Apply the acknowledged placement without rebuilding window identities.
                if(response.placement) root.installed=root.installed.map(function(item) {
                    return item.instanceId===response.updated ? Object.assign({},item,{placement:response.placement}) : item;
                });
                root.refresh();
            }
        }
    }
    function screenFor(name) {
        for (var i=0; i<Quickshell.screens.length; i++) if (Quickshell.screens[i].name === name) return Quickshell.screens[i];
        return !name && Quickshell.screens.length ? Quickshell.screens[0] : null;
    }
    Component.onCompleted: refresh()
    Timer { interval:1000; running:true; repeat:true; onTriggered:root.refresh() }
    IpcHandler {
        target: "io.github.tcballard.widget-core"
        function manage(): void { root.managerOpen = !root.managerOpen; if(root.managerOpen) root.refresh(); }
        function refresh(): bool { return root.refresh(); }
        function show(): void { root.shown = true; }
        function hide(): void { root.shown = false; root.control("finish-arrange"); }
        function arrange(): void { root.shown = true; root.control("arrange"); }
        function status(): string { return JSON.stringify({api:2,version:"0.0.2",installed:root.installed.length,shown:root.shown,editing:root.editing,busy:operation.busy,error:root.error}); }
    }
    PanelWindow {
        visible: root.managerOpen
        implicitWidth: Math.min(Style.space(760),screen ? screen.width-Style.space(32) : Style.space(760))
        implicitHeight: Math.min(Style.space(660),screen ? screen.height-Style.space(32) : Style.space(660))
        color: "transparent"
        exclusionMode: ExclusionMode.Normal
        WlrLayershell.namespace: "tcballard-widget-manager"
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.keyboardFocus: WlrKeyboardFocus.OnDemand
        Core.Manager {
            anchors.fill: parent
            entries: root.installed
            catalog: root.catalog
            retained: root.retained
            workspaceError: root.desktop.error || ""
            onWorkspaceRequested: function(id,workspace) { root.execute(["workspace",id,workspace]); }
            busy: operation.busy
            error: root.error || Object.keys(root.placementErrors).map(function(id) { return root.placementErrors[id]; }).filter(function(message) { return !!message; }).join("; ")
            onConfigureRequested: function(id) { root.control("close-manager");root.managerOpen=false;root.configure(id); }
            onCloseRequested: { root.managerOpen=false;root.control("close-manager"); }
            onRefreshRequested: root.refresh()
            onToggleRequested: function(id, enabled) { root.execute([enabled ? "add" : "hide", id]); }
            onCreateRequested: function(id, family) { root.execute(["create",id,family]); }
            onDuplicateRequested: function(id) { root.execute(["duplicate",id]); }
            onRemoveRequested: function(id) { root.execute(["remove-instance",id]); }
            onUninstallRequested: function(id, policy) { root.execute(["uninstall",id,policy]); }
            onArrangeRequested: { root.control("arrange"); root.shown = true; root.managerOpen = false;root.control("close-manager"); }
        }
    }
    QtObject {
        id: settingsContext
        readonly property var theme: Color
        readonly property var metrics: Style
        property var draftSettings: ({})
        property int revision: 0
        readonly property var appearance: root.themeAppearance
    }
    FloatingWindow {
        title: "Widget settings"
        onClosed: root.closeSettings()
        visible: !root.managerRole && root.configuring!==""
        implicitWidth: Style.space(520)
        implicitHeight: Style.space(560)
        color: "transparent"
        Core.SettingsPanel {
            objectName: "settings-panel"
            anchors.fill: parent
            busy: !!(root.saveStates[root.configuring] && root.saveStates[root.configuring].saving)
            canSave: settingsLoader.status===Loader.Ready
            error: root.editorError || (settingsLoader.status===Loader.Error ? "The widget settings editor could not load. Cancel and check the package." : "") || (root.saveStates[root.configuring] ? root.saveStates[root.configuring].error : "")
            onCancelRequested: root.closeSettings()
            onSaveRequested: if(canSave) root.save(root.configuring,settingsContext.draftSettings,settingsContext.revision)
            Loader { id:settingsLoader; anchors.fill:parent; active:root.configuring!=="" }
        }
    }
    Variants {
        model: root.managerRole ? Quickshell.screens : []
        PanelWindow {
            required property var modelData
            screen: modelData
            visible: root.editing && root.shown
            anchors { top:true; bottom:true; left:true; right:true }
            color: "transparent"
            exclusionMode: ExclusionMode.Ignore
            mask: Region {}
            WlrLayershell.layer: WlrLayer.Bottom
            WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
            Canvas {
                anchors.fill:parent
                property var grid: root.desktop.grids ? root.desktop.grids[modelData.name] : null
                onGridChanged: requestPaint()
                onWidthChanged: requestPaint()
                onHeightChanged: requestPaint()
                onVisibleChanged: requestPaint()
                onPaint: {
                    var ctx=getContext("2d"); ctx.clearRect(0,0,width,height);
                    if(!grid)return;
                    ctx.strokeStyle=Qt.rgba(Color.foreground.r,Color.foreground.g,Color.foreground.b,0.25);
                    ctx.lineWidth=1;
                    for(var col=0;col<grid.columns;col++) for(var row=0;row<grid.rows;row++)
                        ctx.strokeRect(grid.x+col*(grid.cell+grid.gapX),grid.y+row*(grid.cell+grid.gapY),grid.cell,grid.cell);
                }
            }
        }
    }
    Variants {
        model: root.managerRole ? [] : root.installed.filter(function(w) { return w.placement && w.placement.enabled; }).map(function(w) { return w.instanceId; })
        PanelWindow {
            id: window
            required property string modelData
            readonly property var entry: root.entry(modelData)
            readonly property var config: entry ? entry.placement : ({settings:{},x:16,y:16,monitor:"",size:"medium",revision:0})
            readonly property var metadata: entry ? entry.manifest : ({families:["medium"],defaultFamily:"medium",name:"",id:""})
            readonly property string sizeName: metadata.families.indexOf(config.size)>=0 ? config.size : metadata.defaultFamily
            readonly property var effective: entry && entry.effective ? entry.effective : null
            readonly property var grid: effective && root.desktop.grids ? root.desktop.grids[effective.monitor] : null
            readonly property var desired: Grid.geometry(sizeName, grid)
            property real positionX: effective ? effective.x : 0
            property real positionY: effective ? effective.y : 0
            property bool moving: false
            readonly property string geometryKey: JSON.stringify({effective:effective,grid:grid,workspace:config.workspace})
            onGeometryKeyChanged: { moving=false; resetPosition(); }
            function resetPosition() { positionX=effective ? effective.x : 0; positionY=effective ? effective.y : 0; }
            readonly property var target: grid ? Grid.target(positionX,positionY,sizeName,effective.monitor,config.workspace,grid) : null
            readonly property bool validTarget: !!target && Grid.valid(target,grid,root.occupancy.filter(function(_,i) { return i!==entry.occupancyIndex; }))
            screen: root.screenFor(effective ? effective.monitor : "")
            anchors { top: true; left: true }
            margins {
                left: window.moving && window.target ? window.grid.x+window.target.column*(window.grid.cell+window.grid.gapX) : (window.effective ? window.effective.x : 0)
                top: window.moving && window.target ? window.grid.y+window.target.row*(window.grid.cell+window.grid.gapY) : (window.effective ? window.effective.y : 0)
            }
            implicitWidth: desired.width
            implicitHeight: desired.height
            color: "transparent"
            exclusionMode: ExclusionMode.Ignore
            WlrLayershell.namespace: "tcballard-widget-" + modelData
            WlrLayershell.layer: WlrLayer.Bottom
            WlrLayershell.keyboardFocus: root.editing || context.inputRequested ? WlrKeyboardFocus.OnDemand : WlrKeyboardFocus.None
            // No compositor command socket is exposed to the sandbox.
            // Bottom-layer surfaces remain behind fullscreen windows.
            visible: root.shown && effective !== null && screen !== null && Workspace.visible(config.workspace, screen ? screen.name : "", root.desktop)
            function place(size, monitorName) {
                if(!target)return false;
                return root.execute(["place", modelData, JSON.stringify({column:target.column,row:target.row,monitor:monitorName,size:size})]);
            }
            QtObject {
                id: context
                readonly property var settings: window.config.settings
                readonly property bool active: window.visible
                readonly property var theme: Color
                readonly property var metrics: Style
                readonly property int api: 2
                readonly property string instanceId: window.modelData
                readonly property string packageId: window.metadata.id
                readonly property string definitionId: "main"
                readonly property string family: window.sizeName
                readonly property string sizeName: window.metadata.coreApi===1 ? Grid.legacyName(window.sizeName) : window.sizeName
                readonly property var appearance: Object.assign({}, root.themeAppearance, window.config.settings.appearance || {}, root.desktop.frame || {})
                readonly property var saveState: root.saveStates[window.modelData] || ({saving:false,error:"",saved:false})
                readonly property string saveError: saveState.error
                readonly property bool saving: saveState.saving
                readonly property bool saved: saveState.saved
                readonly property int settingsRevision: window.config.revision
                property int draftRevision: settingsRevision
                property bool inputRequested: false
                function requestInput(enabled) { inputRequested = enabled; if(enabled) draftRevision=settingsRevision; }
                function requestConfigure() { root.configure(window.modelData); }
                function saveSettings(value) { return root.save(window.modelData,value,draftRevision); }
                onSettingsRevisionChanged: if(!inputRequested || saved) draftRevision=settingsRevision
            }
            Core.WidgetFrame {
                anchors.fill: parent
                title: window.metadata.name
                appearance: context.appearance
                editing: root.editing
                sizeName: window.sizeName
                monitorName: window.screen ? window.screen.name : ""
                moveStepX: window.grid ? window.grid.cell+window.grid.gapX : 192
                moveStepY: window.grid ? window.grid.cell+window.grid.gapY : 192
                invalidTarget: window.moving && !window.validTarget
                notice: (window.moving && !window.validTarget ? "Those cells are unavailable" : "") || root.placementErrors[window.modelData] || context.saveError || (context.saving ? "Saving…" : "")
                configurable: !!window.metadata.settingsEntryPoint
                onConfigureRequested: root.configure(window.modelData)
                onEditRequested: root.control("arrange")
                onEscapeRequested: root.control("finish-arrange")
                onHideRequested: root.execute(["hide", window.modelData])
                onMoved: function(dx,dy) { window.moving=true; window.positionX += dx; window.positionY += dy; }
                onFinishedMoving: {
                    if(window.validTarget) window.place(window.sizeName,window.effective.monitor);
                    else { var errors=Object.assign({},root.placementErrors);errors[window.modelData]="Those cells are unavailable; placement unchanged";root.placementErrors=errors; }
                    window.moving=false;window.resetPosition();
                }
                onSizeRequested: { var sizes=window.metadata.families; window.place(sizes[(sizes.indexOf(window.sizeName)+1)%sizes.length],window.effective.monitor); }
                onMonitorRequested: { var screens=Quickshell.screens; if(screens.length) window.place(window.sizeName,screens[(screens.indexOf(window.screen)+1)%screens.length].name); }
                Loader {
                    id: content
                    anchors.fill: parent
                    active: window.visible
                    readonly property string entryUrl: window.entry ? "file://" + window.entry.directory.split("/").map(encodeURIComponent).join("/") + "/" + window.metadata.entryPoint.split("/").map(encodeURIComponent).join("/") : ""
                    onEntryUrlChanged: if(entryUrl) setSource(entryUrl,{widgetContext:context})
                    Component.onCompleted: if(entryUrl) setSource(entryUrl,{widgetContext:context})
                }
                Core.Label { anchors.centerIn: parent; visible: content.status === Loader.Error; text: "Widget could not load"; color: Color.urgent }
            }
        }
    }
}
