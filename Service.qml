import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Hyprland
import qs.Commons
import "qml" as Core

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
    property bool refreshing: false
    property bool timedOut: false
    readonly property string helper: decodeURIComponent(Qt.resolvedUrl("bin/omarchy-widget").toString().replace(/^file:\/\//, ""))
    function execute(args) {
        if (operation.running) return false;
        timedOut = false;
        refreshing = args[0] === "list";
        operation.command = ["/usr/bin/timeout", "--kill-after=1", "10", root.helper].concat(args);
        operation.running = true;
        watchdog.restart();
        return true;
    }
    function refresh() { return execute(["list"]); }
    function complete(text, code) {
        watchdog.stop();
        if (timedOut) return;
        try {
            if (text.length > 2097152) throw new Error("Registry response too large");
            var response = JSON.parse(text);
            if (code !== 0) throw new Error(response.error || "Registry operation failed");
            error = "";
            if (refreshing) {
                if (response.api !== 1 || !Array.isArray(response.installed)) throw new Error("Unsupported registry response");
                installed = response.installed;
                themeAppearance = response.appearance || {};
                error = (response.problems || []).join("; ");
            } else Qt.callLater(refresh);
        } catch (e) { error = "" + (e.message || "Core helper unavailable. Run bash install-local."); }
    }
    function screenFor(name) {
        for (var i=0; i<Quickshell.screens.length; i++) if (Quickshell.screens[i].name === name) return Quickshell.screens[i];
        return Quickshell.screens.length ? Quickshell.screens[0] : null;
    }
    Process {
        id: operation
        stdout: StdioCollector { id: output; waitForEnd: true }
        onExited: function(code, status) { root.complete(output.text, code); }
    }
    Timer { id: watchdog; interval: 12000; onTriggered: { root.timedOut = true; operation.running = false; root.error = "Core operation timed out. Refresh to retry."; } }
    Component.onCompleted: refresh()
    IpcHandler {
        target: "io.github.tcballard.widget-core"
        function manage(): void { root.managerOpen = !root.managerOpen; if(root.managerOpen) root.refresh(); }
        function refresh(): bool { return root.refresh(); }
        function show(): void { root.shown = true; }
        function hide(): void { root.shown = false; root.editing = false; }
        function arrange(): void { root.shown = true; root.editing = !root.editing; }
        function status(): string { return JSON.stringify({api:1,installed:root.installed.length,shown:root.shown,editing:root.editing,busy:operation.running,error:root.error}); }
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
            busy: operation.running
            error: root.error
            onCloseRequested: root.managerOpen = false
            onRefreshRequested: root.refresh()
            onToggleRequested: function(id, enabled) { root.execute([enabled ? "add" : "hide", id]); }
            onArrangeRequested: { root.editing = !root.editing; root.shown = true; root.managerOpen = false; }
        }
    }
    Variants {
        model: root.installed.filter(function(w) { return w.placement && w.placement.enabled; })
        PanelWindow {
            id: window
            required property var modelData
            readonly property var config: modelData.placement
            readonly property var metadata: modelData.manifest
            readonly property string sizeName: metadata.sizes[config.size] ? config.size : metadata.defaultSize
            readonly property var desired: metadata.sizes[sizeName]
            property real positionX: config.x
            property real positionY: config.y
            readonly property real scale: Style.spaceReal(1)
            screen: root.screenFor(config.monitor)
            anchors { top: true; left: true }
            margins {
                left: Math.max(0, Math.min(window.positionX, (window.screen ? window.screen.width : 1920) - window.width))
                top: Math.max(0, Math.min(window.positionY, (window.screen ? window.screen.height : 1080) - window.height))
            }
            implicitWidth: Math.min(desired.width * scale, screen ? screen.width : 1920)
            implicitHeight: Math.min((desired.height + (root.editing ? 42 : 0)) * scale, screen ? screen.height : 1080)
            color: "transparent"
            exclusionMode: ExclusionMode.Ignore
            WlrLayershell.namespace: "tcballard-widget-" + metadata.id
            WlrLayershell.layer: WlrLayer.Bottom
            WlrLayershell.keyboardFocus: root.editing || context.inputRequested ? WlrKeyboardFocus.OnDemand : WlrKeyboardFocus.None
            readonly property var monitor: screen ? Hyprland.monitorFor(screen) : null
            readonly property bool obscured: monitor && monitor.activeWorkspace ? monitor.activeWorkspace.hasFullscreen : false
            visible: root.shown && screen !== null && !obscured
            function place(size, monitorName) {
                return root.execute(["place", metadata.id, JSON.stringify({x:Math.max(0,margins.left),y:Math.max(0,margins.top),monitor:monitorName,size:size})]);
            }
            QtObject {
                id: context
                readonly property var settings: window.config.settings
                readonly property bool active: window.visible
                readonly property string sizeName: window.sizeName
                readonly property var appearance: Object.assign({}, root.themeAppearance, window.config.settings.appearance || {})
                readonly property string saveError: root.error
                readonly property bool saving: operation.running
                property bool inputRequested: false
                function requestInput(enabled) { inputRequested = enabled; }
                function saveSettings(value) { return root.execute(["configure", window.metadata.id, JSON.stringify(value)]); }
            }
            Core.WidgetFrame {
                anchors.fill: parent
                title: window.metadata.name
                appearance: context.appearance
                editing: root.editing
                sizeName: window.sizeName
                monitorName: window.screen ? window.screen.name : ""
                notice: root.error
                onEditRequested: root.editing = !root.editing
                onEscapeRequested: root.editing = false
                onHideRequested: root.execute(["hide", window.metadata.id])
                onMoved: function(dx,dy) { window.positionX = Math.max(0,window.margins.left+dx); window.positionY = Math.max(0,window.margins.top+dy); }
                onFinishedMoving: window.place(window.sizeName, window.config.monitor)
                onSizeRequested: { var sizes=Object.keys(window.metadata.sizes); window.place(sizes[(sizes.indexOf(window.sizeName)+1)%sizes.length],window.config.monitor); }
                onMonitorRequested: { var screens=Quickshell.screens; if(screens.length) window.place(window.sizeName,screens[(screens.indexOf(window.screen)+1)%screens.length].name); }
                Loader {
                    id: content
                    anchors.fill: parent
                    active: window.visible
                    Component.onCompleted: setSource("file://" + window.modelData.directory.split("/").map(encodeURIComponent).join("/") + "/" + window.metadata.entryPoint.split("/").map(encodeURIComponent).join("/"), {widgetContext:context})
                }
                Core.Label { anchors.centerIn: parent; visible: content.status === Loader.Error; text: "Widget could not load"; color: Color.urgent }
            }
        }
    }
}
