import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Hyprland
import qs.Commons
import "qml" as Core
import "qml/Grid.js" as Grid

Item {
    id: root
    property var shell: null
    property var manifest: null
    property string omarchyPath: ""
    property var installed: []
    property var themeAppearance: ({})
    property string error: ""
    property bool shown: true
    property bool editing: false
    property bool managerOpen: false
    property var saveStates: ({})
    property string configuring: ""
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
    function save(id, value, revision) {
        if (saveStates[id] && saveStates[id].saving) return false;
        if (!execute(["save",id,JSON.stringify({revision:revision,settings:value})],id)) return false;
        var states=Object.assign({},saveStates); states[id]={saving:true,error:"",saved:false}; saveStates=states;
        return true;
    }
    function configure(id) {
        var item=entry(id);
        if(!item || !item.manifest.settingsEntryPoint) { error="This widget provides its own settings editor."; return; }
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
                var states=Object.assign({},root.saveStates);
                states[request.token]={saving:false,error:message,saved:success}; root.saveStates=states;
                if(success && root.configuring===request.token) root.configuring="";
            }
            if(!success) return;
            if(request.args[0] === "list") {
                if(response.api !== 2 || !Array.isArray(response.installed)) { root.error="Unsupported Core response"; return; }
                root.installed=response.installed;
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
        return Quickshell.screens.length ? Quickshell.screens[0] : null;
    }
    Component.onCompleted: refresh()
    Timer { interval:5000; running:true; repeat:true; onTriggered:root.refresh() }
    IpcHandler {
        target: "io.github.tcballard.widget-core"
        function manage(): void { root.managerOpen = !root.managerOpen; if(root.managerOpen) root.refresh(); }
        function refresh(): bool { return root.refresh(); }
        function show(): void { root.shown = true; }
        function hide(): void { root.shown = false; root.editing = false; }
        function arrange(): void { root.shown = true; root.editing = !root.editing; }
        function status(): string { return JSON.stringify({api:2,version:"0.0.2",installed:root.installed.length,shown:root.shown,editing:root.editing,busy:operation.busy,error:root.error}); }
    }
    PanelWindow {
        visible: root.managerOpen
        implicitWidth: Style.space(540)
        implicitHeight: Style.space(500)
        color: "transparent"
        exclusionMode: ExclusionMode.Ignore
        WlrLayershell.namespace: "tcballard-widget-manager"
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.keyboardFocus: WlrKeyboardFocus.OnDemand
        Core.Manager {
            anchors.fill: parent
            entries: root.installed
            busy: operation.busy
            error: root.error
            onCloseRequested: root.managerOpen = false
            onRefreshRequested: root.refresh()
            onToggleRequested: function(id, enabled) { root.execute([enabled ? "add" : "hide", id]); }
            onArrangeRequested: { root.editing = !root.editing; root.shown = true; root.managerOpen = false; }
        }
    }
    QtObject {
        id: settingsContext
        property var draftSettings: ({})
        property int revision: 0
        readonly property var appearance: root.themeAppearance
    }
    PanelWindow {
        visible: root.configuring!==""
        implicitWidth: Style.space(520)
        implicitHeight: Style.space(560)
        color: "transparent"
        exclusionMode: ExclusionMode.Ignore
        WlrLayershell.namespace: "tcballard-widget-settings"
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.keyboardFocus: WlrKeyboardFocus.OnDemand
        Core.SettingsPanel {
            anchors.fill: parent
            busy: !!(root.saveStates[root.configuring] && root.saveStates[root.configuring].saving)
            error: root.saveStates[root.configuring] ? root.saveStates[root.configuring].error : ""
            onCancelRequested: root.configuring=""
            onSaveRequested: root.save(root.configuring,settingsContext.draftSettings,settingsContext.revision)
            Loader { id:settingsLoader; anchors.fill:parent; active:root.configuring!=="" }
        }
    }
    Variants {
        model: Quickshell.screens
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
                property real spacing: Style.space(16)
                onWidthChanged: requestPaint()
                onHeightChanged: requestPaint()
                onSpacingChanged: requestPaint()
                onVisibleChanged: requestPaint()
                onPaint: {
                    var ctx=getContext("2d"); ctx.clearRect(0,0,width,height);
                    ctx.fillStyle=Qt.rgba(Color.foreground.r,Color.foreground.g,Color.foreground.b,0.18);
                    for(var x=spacing;x<width;x+=spacing) for(var y=spacing;y<height;y+=spacing) ctx.fillRect(x,y,1.5,1.5);
                }
            }
        }
    }
    Variants {
        model: root.installed.filter(function(w) { return w.placement && w.placement.enabled; }).map(function(w) { return w.instanceId; })
        PanelWindow {
            id: window
            required property string modelData
            readonly property var entry: root.entry(modelData)
            readonly property var config: entry ? entry.placement : ({settings:{},x:16,y:16,monitor:"",size:"medium",revision:0})
            readonly property var metadata: entry ? entry.manifest : ({families:["medium"],defaultFamily:"medium",name:"",id:""})
            readonly property string sizeName: metadata.families.indexOf(config.size)>=0 ? config.size : metadata.defaultFamily
            readonly property var desired: Grid.geometry(sizeName)
            property real positionX: config.x
            property real positionY: config.y
            readonly property real scale: Style.spaceReal(1)
            screen: root.screenFor(config.monitor)
            anchors { top: true; left: true }
            margins {
                left: Grid.snap(window.positionX, (window.screen ? window.screen.width : 1920)/window.scale, window.desired.width)*window.scale
                top: Grid.snap(window.positionY, (window.screen ? window.screen.height : 1080)/window.scale, window.desired.height)*window.scale
            }
            implicitWidth: Math.min(desired.width * scale, screen ? screen.width : 1920)
            implicitHeight: Math.min(desired.height * scale, screen ? screen.height : 1080)
            color: "transparent"
            exclusionMode: ExclusionMode.Ignore
            WlrLayershell.namespace: "tcballard-widget-" + modelData
            WlrLayershell.layer: WlrLayer.Bottom
            WlrLayershell.keyboardFocus: root.editing || context.inputRequested ? WlrKeyboardFocus.OnDemand : WlrKeyboardFocus.None
            readonly property var monitor: screen ? Hyprland.monitorFor(screen) : null
            readonly property bool obscured: monitor && monitor.activeWorkspace ? monitor.activeWorkspace.hasFullscreen : false
            visible: root.shown && screen !== null && !obscured
            function place(size, monitorName) {
                return root.execute(["place", modelData, JSON.stringify({x:margins.left/scale,y:margins.top/scale,monitor:monitorName,size:size})]);
            }
            QtObject {
                id: context
                readonly property var settings: window.config.settings
                readonly property bool active: window.visible
                readonly property int api: 2
                readonly property string instanceId: window.modelData
                readonly property string packageId: window.metadata.id
                readonly property string definitionId: "main"
                readonly property string family: window.sizeName
                readonly property string sizeName: window.metadata.coreApi===1 ? Grid.legacyName(window.sizeName) : window.sizeName
                readonly property var appearance: Object.assign({}, root.themeAppearance, window.config.settings.appearance || {})
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
                notice: context.saveError || (context.saving ? "Saving…" : "")
                configurable: !!window.metadata.settingsEntryPoint
                onConfigureRequested: root.configure(window.modelData)
                onEditRequested: root.editing = !root.editing
                onEscapeRequested: root.editing = false
                onHideRequested: root.execute(["hide", window.modelData])
                onMoved: function(dx,dy) { window.positionX = Math.max(0,window.positionX+dx/window.scale); window.positionY = Math.max(0,window.positionY+dy/window.scale); }
                onFinishedMoving: window.place(window.sizeName, window.config.monitor)
                onSizeRequested: { var sizes=window.metadata.families; window.place(sizes[(sizes.indexOf(window.sizeName)+1)%sizes.length],window.config.monitor); }
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
