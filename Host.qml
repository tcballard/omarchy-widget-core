import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import qs.Commons
import "qml" as Core
import "qml/Grid.js" as Grid
import "qml/Workspace.js" as Workspace
import "qml/Declarative.js" as Declarative

Item {
    id: root
    property var shell: null
    property var manifest: null
    property string omarchyPath: ""
    readonly property bool managerRole: Quickshell.env("OMARCHY_WIDGET_ROLE") === "manager"
    readonly property bool declarativeWanted: managerRole && Declarative.wanted(installed,catalog,currentEditor ? currentEditor.instance : "",shown)
    property var clockTimes: ({})
    property int sampledMinute: -1
    property bool clockPending: false
    property string sampledZones: ""
    readonly property var clockZones: managerRole ? Declarative.zones(installed,catalog,shown,desktop) : []
    function refreshClocks() {
        var zones=JSON.stringify(clockZones), minute=Math.floor(Date.now()/60000);
        if(!clockZones.length || clockPending || (minute===sampledMinute && zones===sampledZones)) return;
        clockPending=execute(["clock-times",zones],{clockMinute:minute,clockZones:zones});
    }
    onClockZonesChanged:refreshClocks()
    property string editSerial: ""
    property var pendingClose: null
    property string queuedConfigure: ""
    property var contentFailures: ({})
    property var currentEditor: null
    property bool repairRequired: false
    property bool repairDismissed: false
    property string deliveryError: ""
    property var installed: []
    property var catalog: []
    property var retained: []
    property var occupancy: []
    property var desktop: ({available:false,monitors:{}})
    property var themeAppearance: ({})
    property string error: ""
    property string notice: ""
    readonly property bool revealRunner: Quickshell.env("OMARCHY_WIDGET_REVEAL") === "1"
    property bool revealing: false
    property bool shown: true
    property bool editing: false
    property bool managerOpen: false
    property bool snapshotReady: false
    property var weatherStates: ({})
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
    function acknowledgeClose() {
        pendingClose=null;deliveryError="";
        var next=queuedConfigure;queuedConfigure="";
        if(next) configure(next);
    }
    function retryDeliveries() {
        if(pendingClose && !pendingClose.inFlight) {
            var close=Object.assign({},pendingClose,{inFlight:true});
            pendingClose=close;
            if(!execute(["edit-done",close.instance,close.serial],{delivery:"close",serial:close.serial})) pendingClose=Object.assign({},close,{inFlight:false});
        }
        Object.keys(contentFailures).forEach(function(id) {
            var failure=root.contentFailures[id];
            if(failure.inFlight)return;
            var failures=Object.assign({},root.contentFailures);
            failures[id]={inFlight:true};root.contentFailures=failures;
            if(!root.execute(["content-failed",id],{delivery:"content",instance:id})) {
                failures=Object.assign({},root.contentFailures);failures[id]={inFlight:false};root.contentFailures=failures;
            }
        });
    }
    function contentFailed(id) {
        var failures=Object.assign({},contentFailures);
        if(!failures[id]) failures[id]={inFlight:false};
        contentFailures=failures;
        retryDeliveries();
    }
    Timer { interval:500; running:root.pendingClose!==null || Object.keys(root.contentFailures).length>0; repeat:true; onTriggered:root.retryDeliveries() }
    function fileUrl(directory,path) { return "file://"+directory.split("/").concat(path.split("/")).map(encodeURIComponent).join("/"); }
    function control(method) { return execute(["control",method]); }
    function save(id, value, revision, action) {
        if (saveStates[id] && saveStates[id].saving) return false;
        if(configuring===id) editorError="";
        var token={instance:id,generation:!action && configuring===id?settingsGeneration:-1};
        if (!execute(["save",id,JSON.stringify({revision:revision,settings:value})],token)) {
            if(configuring===id) editorError=error;
            return false;
        }
        var states=Object.assign({},saveStates); states[id]={saving:true,error:"",saved:false}; saveStates=states;
        return true;
    }
    function closeSettings() {
        if(configuring && editSerial && entry(configuring)) {
            pendingClose={instance:configuring,serial:editSerial,inFlight:false};
            retryDeliveries();
        }
        configuring="";
        settingsGeneration++;
        settingsLoader.source="";
        settingsContext.draftSettings=({});
        editorError="";
    }
    function configure(id) {
        if(root.revealRunner) return false;
        if(pendingClose) { queuedConfigure=id;deliveryError="Finishing the previous settings window. Retrying automatically…"; return true; }
        if(configuring && configuring!==id) {editorError="Save or cancel this editor before configuring another widget."; return;}
        if(configuring===id) return;
        execute(["edit",id]);
    }
    function openSettings(id) {
        // Repeated gear presses must not reset an unsaved draft.
        if(configuring===id) return;
        if(configuring!=="") { editorError="Save or cancel this editor before configuring another widget."; return; }
        if(saveStates[id] && saveStates[id].saving) { error="This widget is still saving. Please retry after it finishes."; return; }
        var item=entry(id);
        if(!item || !item.placement || (!item.manifest.settingsEntryPoint && !Declarative.isDeclarative(item))) { error="This widget has no available settings editor."; return; }
        settingsGeneration++;
        editorError="";
        var states=Object.assign({},saveStates); delete states[id]; saveStates=states;
        settingsContext.draftSettings=JSON.parse(JSON.stringify(item.placement.settings));
        settingsContext.revision=item.placement.revision;
        configuring=id;
        if(Declarative.isDeclarative(item)) settingsLoader.setSource(Qt.resolvedUrl("qml/DeclarativeSettings.qml"),{settingsContext:settingsContext,definition:item.manifest});
        else settingsLoader.setSource(fileUrl(item.directory,item.manifest.settingsEntryPoint),{settingsContext:settingsContext});
        return true;
    }
    Core.RegistryQueue {
        id: operation
        helper: root.helper
        onCompleted: function(request,success,response,message) {
            if(request.args[0]==="clock-times") {
                root.clockPending=false;
                if(success) {root.clockTimes=response.zones || {};root.sampledMinute=request.token.clockMinute;root.sampledZones=request.token.clockZones;}
                return;
            }
            if(request.args[0]==="weather") {
                var weather=Object.assign({},root.weatherStates);
                weather[request.args[1]]=success ? response : {state:"unavailable",data:null,error:message};
                root.weatherStates=weather; return;
            }
            if(request.token && request.token.delivery) {
                if(request.token.delivery==="close" && root.pendingClose && root.pendingClose.serial===request.token.serial) {
                    if(success) root.acknowledgeClose();
                    else root.pendingClose=Object.assign({},root.pendingClose,{inFlight:false});
                } else if(request.token.delivery==="content") {
                    var failures=Object.assign({},root.contentFailures);
                    if(success || message==="Runner generation expired") delete failures[request.token.instance];
                    else failures[request.token.instance]={inFlight:false};
                    root.contentFailures=failures;
                }
                root.deliveryError=success ? "" : "Recovery acknowledgement pending. Retrying automatically: "+message;
                root.refresh();
                return;
            }
            root.error=message;
            if(success && (request.args[0]==="export-settings" || request.args[0]==="repair")) root.notice=response.message;
            if(request.token) {
                var id=request.token.instance;
                var states=Object.assign({},root.saveStates);
                states[id]={saving:false,error:message,saved:success}; root.saveStates=states;
                if(success && root.configuring===id && root.settingsGeneration===request.token.generation) root.closeSettings();
            }
            if(request.args[0]==="place" || request.args[0]==="workspace" || request.args[0]==="recover-placement") {
                var placementErrors=Object.assign({},root.placementErrors);
                placementErrors[request.args[1]]=message; root.placementErrors=placementErrors;
            }
            if(!success) { if(request.args[0]!=="list")root.refresh(); return; }
            if(request.args[0] === "list") {
                if(response.api !== 2 || !Array.isArray(response.installed)) { root.error="Unsupported Core response"; return; }
                root.snapshotReady=true;
                // Preserve delegate focus and in-progress manager edits during polling.
                if(JSON.stringify(root.installed)!==JSON.stringify(response.installed)) root.installed=response.installed;
                if(root.configuring!=="" && !root.entry(root.configuring)) root.closeSettings();
                if(JSON.stringify(root.catalog)!==JSON.stringify(response.catalog || [])) root.catalog=response.catalog || [];
                if(JSON.stringify(root.retained)!==JSON.stringify(response.retained || [])) root.retained=response.retained || [];
                root.repairRequired=response.repairRequired === true;
                if(!root.repairRequired) root.repairDismissed=false;
                var outstanding=Object.assign({},root.contentFailures);
                Object.keys(outstanding).forEach(function(id) { if(!root.entry(id) || !root.entry(id).placement) delete outstanding[id]; });
                root.contentFailures=outstanding;
                root.occupancy=response.occupancy || [];
                var desktop=response.desktop || ({available:false,monitors:{}});
                if(JSON.stringify(root.desktop)!==JSON.stringify(desktop)) root.desktop=desktop;
                if(response.runtime) {
                    root.revealing=response.runtime.revealing === true;
                    root.shown=response.runtime.shown !== false;
                    root.editing=response.runtime.editing === true;
                    root.managerOpen=root.managerRole && (response.runtime.managerOpen === true || (root.repairRequired && !root.repairDismissed));
                    var edit=response.runtime.edit;
                    root.currentEditor=edit || null;
                    if(root.pendingClose && (!edit || edit.instance!==root.pendingClose.instance || String(edit.serial)!==root.pendingClose.serial)) {
                        root.acknowledgeClose();
                    }
                    if(root.configuring!=="" && (!edit || edit.instance!==root.configuring || String(edit.serial)!==root.editSerial)) root.closeSettings();
                    if(!root.pendingClose && root.configuring==="" && edit && (!root.managerRole || Declarative.isDeclarative(root.entry(edit.instance)))) {
                        if(root.openSettings(edit.instance)) root.editSerial=String(edit.serial);
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
    Timer { interval:1000; running:true; repeat:true; onTriggered:{root.refresh();root.refreshClocks();} }
    // Wait for queued Configure/Save/close acknowledgements before releasing Qt.
    Timer {
        interval:250
        running:root.managerRole && !root.declarativeWanted && root.configuring==="" && root.snapshotReady && !root.managerOpen && !root.editing && !root.revealing && !operation.busy && root.pendingClose===null && Object.keys(root.contentFailures).length===0
        onTriggered:Qt.quit()
    }
    IpcHandler {
        target: "io.github.tcballard.widget-core"
        function reveal(): void { root.control("reveal"); }
        function manage(): void { root.managerOpen = !root.managerOpen; if(root.managerOpen) root.refresh(); }
        function refresh(): bool { return root.refresh(); }
        function show(): void { root.shown = true; }
        function hide(): void { root.shown = false; root.control("finish-arrange"); }
        function arrange(): void { root.shown = true; root.control("arrange"); }
        function status(): string { return JSON.stringify({api:2,version:"0.0.2",installed:root.installed.length,shown:root.shown,editing:root.editing,busy:operation.busy,error:root.error}); }
    }
    // Trusted dismissal surface. Widget code remains in its package process.
    PanelWindow {
        visible: root.managerRole && root.revealing
        anchors { top:true; bottom:true; left:true; right:true }
        color: "transparent"
        exclusionMode: ExclusionMode.Ignore
        WlrLayershell.namespace: "widget-core-reveal-dismiss"
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive
        Item {
            anchors.fill:parent
            focus:true
            Keys.onEscapePressed: root.control("dismiss-reveal")
            MouseArea { anchors.fill:parent; onClicked:root.control("dismiss-reveal") }
        }
    }
    FloatingWindow {
        title: "Widget Manager"
        visible: root.managerOpen
        onClosed: {
            root.repairDismissed=true;
            root.control("close-manager");
            root.managerOpen=false;
        }
        implicitWidth: Math.min(Style.space(760),screen ? screen.width-Style.space(32) : Style.space(760))
        implicitHeight: Math.min(Style.space(660),screen ? screen.height-Style.space(32) : Style.space(660))
        color: "transparent"
        Loader {
            anchors.fill:parent
            active:root.managerRole && root.managerOpen
            sourceComponent:Component {
                Core.Manager {
                    monitors: Object.keys(root.desktop.grids || {})
                    notice: root.notice
                    pendingEditor: root.currentEditor
                    repairRequired: root.repairRequired
                    onCancelEditorRequested: function(id,serial) { root.execute(["edit-done",id,serial]); }
                    onRepairRequested: root.execute(["repair"])
                    onRecoverRequested: function(id,size,monitor) { root.execute(["recover-placement",id,JSON.stringify({size:size,monitor:monitor})]); }
                    onExportRequested: function(id) { root.execute(["export-settings",id]); }
                    anchors.fill: parent
                    entries: root.installed
                    catalog: root.catalog
                    retained: root.retained
                    workspaceError: root.desktop.error || ""
                    onWorkspaceRequested: function(id,workspace) { root.execute(["workspace",id,workspace]); }
                    busy: operation.busy
                    error: root.deliveryError || root.error || Object.keys(root.placementErrors).map(function(id) { return root.placementErrors[id]; }).filter(function(message) { return !!message; }).join("; ")
                    onConfigureRequested: function(id) { root.control("close-manager");root.configure(id);root.managerOpen=false; }
                    onCloseRequested: { root.repairDismissed=true;root.control("close-manager");root.managerOpen=false; }
                    onRefreshRequested: root.refresh()
                    onToggleRequested: function(id, enabled) { root.execute([enabled ? "show" : "hide", id]); }
                    onCreateRequested: function(id, family) { root.execute(["create",id,family]); }
                    onInstallRequested: function(path) { root.execute(["install",path]); }
                    onDuplicateRequested: function(id) { root.execute(["duplicate",id]); }
                    onRemoveRequested: function(id) { root.execute(["remove-instance",id]); }
                    onPackageControlRequested: function(id,action) { root.execute(["package-control",id,action]); }
                    onRollbackRequested: function(id) { root.execute(["rollback",id]); }
                    onWeatherPermissionRequested: function(id, allowed) { root.execute(["weather-permission",id,allowed ? "allow" : "deny"]); }
                    onUninstallRequested: function(id, policy) { root.execute(["uninstall",id,policy]); }
                    onArrangeRequested: { root.control("arrange"); root.shown = true;root.control("close-manager");root.managerOpen = false; }
                }
        // manager-content-end
            }
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
        visible: root.configuring!==""
        implicitWidth: Style.space(520)
        implicitHeight: Style.space(560)
        color: "transparent"
        Core.SettingsPanel {
            objectName: "settings-panel"
            anchors.fill: parent
            busy: !!(root.saveStates[root.configuring] && root.saveStates[root.configuring].saving)
            canSave: settingsLoader.status===Loader.Ready && !!settingsLoader.item && !(settingsLoader.item.validationError || "")
            error: root.editorError || (settingsLoader.item ? settingsLoader.item.validationError || "" : "") || (settingsLoader.status===Loader.Error ? "The widget settings editor could not load. Cancel and check the package." : "") || (root.saveStates[root.configuring] ? root.saveStates[root.configuring].error : "")
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
            WlrLayershell.layer: root.revealRunner ? WlrLayer.Overlay : WlrLayer.Bottom
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
        model: root.installed.filter(function(w) { return w.placement && w.placement.enabled && (root.managerRole ? Declarative.isDeclarative(w) && Declarative.enabled(w,root.catalog) : !Declarative.isDeclarative(w)); }).map(function(w) { return w.instanceId; })
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
            readonly property var selectedScreen: root.screenFor(effective ? effective.monitor : "")
            screen: selectedScreen
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
            WlrLayershell.layer: root.revealRunner ? WlrLayer.Overlay : WlrLayer.Bottom
            WlrLayershell.keyboardFocus: !root.revealRunner ? WlrKeyboardFocus.OnDemand : WlrKeyboardFocus.None
            // No compositor command socket is exposed to the sandbox.
            // Bottom-layer surfaces remain behind fullscreen windows.
            visible: (root.shown || root.revealRunner) && effective !== null && selectedScreen !== null && Workspace.visible(config.workspace, selectedScreen ? selectedScreen.name : "", root.desktop)
            function place(size, monitorName) {
                if(!target)return false;
                return root.execute(["place", modelData, JSON.stringify({column:target.column,row:target.row,monitor:monitorName,size:size})]);
            }
            Core.Lifecycle {
                id: context
                readonly property var settings: window.config.settings
                active: window.visible
                readonly property var weather: root.weatherStates[window.modelData] || ({state:"loading",data:null,error:""})
                function requestWeather(latitude,longitude) {
                    return active && root.execute(["weather",window.modelData,String(latitude),String(longitude)]);
                }
                readonly property var theme: Color
                readonly property var metrics: Style
                readonly property int api: 3
                readonly property string instanceId: window.modelData
                readonly property string packageId: window.metadata.id
                readonly property string definitionId: "main"
                readonly property string family: window.sizeName
                readonly property var appearance: Object.assign({}, root.themeAppearance, root.desktop.frame || {})
                readonly property var saveState: root.saveStates[window.modelData] || ({saving:false,error:"",saved:false})
                readonly property string saveError: saveState.error
                readonly property bool saving: saveState.saving
                readonly property bool saved: saveState.saved
                readonly property int settingsRevision: window.config.revision
                function requestConfigure() { root.configure(window.modelData); }
                // API 3 actions use the revision observed when the action was prepared.
                function saveSettings(value, revision) { return root.save(window.modelData,value,revision === undefined ? settingsRevision : revision, true); }
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
                notice: (window.moving && !window.validTarget ? "Those cells are unavailable" : "") || root.placementErrors[window.modelData] || root.deliveryError || context.saveError || (context.saving ? "Saving…" : "")
                configurable: window.metadata.coreApi>=3 && (!!window.metadata.settingsEntryPoint || Declarative.isDeclarative(window.entry))
                onConfigureRequested: root.configure(window.modelData)
                onEditRequested: root.control("finish-arrange")
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
                    onStatusChanged: if(status===Loader.Error) root.contentFailed(window.modelData);
                    anchors.fill: parent
                    active: true
                    readonly property string entryUrl: Declarative.isDeclarative(window.entry) ? Qt.resolvedUrl("qml/DeclarativeView.qml").toString() : window.entry ? "file://" + window.entry.directory.split("/").map(encodeURIComponent).join("/") + "/" + window.metadata.entryPoint.split("/").map(encodeURIComponent).join("/") : ""
                    function loadView() {
                        if(!entryUrl)return;
                        if(Declarative.isDeclarative(window.entry))setSource(entryUrl,{widgetContext:context,definition:window.metadata,clockTimes:root.clockTimes});
                        else setSource(entryUrl,{widgetContext:context});
                    }
                    onEntryUrlChanged:loadView()
                    Component.onCompleted:loadView()
                    Connections {
                        target:root
                        function onClockTimesChanged() {if(content.item && Declarative.isDeclarative(window.entry))content.item.clockTimes=root.clockTimes;}
                    }
                    Connections {
                        target:window
                        function onMetadataChanged() {if(content.item && Declarative.isDeclarative(window.entry))content.item.definition=window.metadata;}
                    }
                }
                Core.Label { anchors.centerIn: parent; visible: content.status === Loader.Error; text: "Widget could not load"; color: Color.urgent }
            }
        }
    }
}
