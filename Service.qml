import QtQuick
import Quickshell.Io

// Optional compatibility bridge. No widget code or windows live in the shell.
Item {
    id: root
    property var shell: null
    property var manifest: null
    property string omarchyPath: ""
    property var pending: []
    readonly property string launcher: decodeURIComponent(Qt.resolvedUrl("widget").toString().replace(/^file:\/\//,""))
    function send(method) {
        if(pending.length>=16)return false;
        var queue=pending.slice();queue.push(method);pending=queue;
        Qt.callLater(pump);return true;
    }
    function pump() {
        if(command.running || !pending.length)return;
        var queue=pending.slice();var method=queue.shift();pending=queue;
        command.command=["/usr/bin/bash",launcher,method];command.running=true;
    }
    Process { id:command; onExited:Qt.callLater(root.pump) }
    Component.onCompleted: send("start")
    IpcHandler {
        target:"io.github.tcballard.widget-core"
        function manage(): bool { return root.send("manage"); }
        function refresh(): bool { return root.send("refresh"); }
        function show(): bool { return root.send("show"); }
        function hide(): bool { return root.send("hide-all"); }
        function arrange(): bool { return root.send("arrange"); }
        function status(): string { return "Widget host is separate. Run: omarchy-widget status"; }
    }
}
