import QtQuick
import Quickshell.Io

// One process, FIFO writes, coalesced placement and refresh requests.
Item {
    id: root
    property string helper: ""
    property var pending: []
    property var current: null
    readonly property bool busy: current !== null || pending.length > 0
    signal completed(var request, bool success, var response, string message)
    function enqueue(args, token) {
        var queue = pending.slice();
        if (args[0] === "list" || args[0] === "place" || args[0] === "weather") {
            for (var i=0;i<queue.length;i++) {
                if (queue[i].args[0] === args[0] && queue[i].args[1] === args[1]) {
                    queue[i] = {args:args,token:token || ""}; pending=queue; return true;
                }
            }
        }
        if (queue.length >= 64) return false;
        queue.push({args:args,token:token || ""}); pending=queue;
        Qt.callLater(pump);
        return true;
    }
    function pump() {
        if (current !== null || process.running || retryDelay.running || !pending.length) return;
        var queue=pending.slice(); current=queue.shift(); pending=queue;
        process.command=["/usr/bin/timeout","--kill-after=1","10",helper].concat(current.args);
        process.running=true;
        watchdog.restart();
    }
    function finish(text, code) {
        if (current === null) return;
        watchdog.stop();
        var request=current;
        var response={}; var message="";
        try {
            if (text.length>2097152) throw new Error("Registry response exceeds limit");
            response=JSON.parse(text);
            if (code !== 0) throw new Error(response.error || "Core operation failed");
        } catch(e) { message=String(e.message || e); }
        if(message && (response.code === "busy" || response.code === "rate_limited") && (request.retries || 0)<4) {
            request.retries=(request.retries || 0)+1;
            current=null;
            var queue=pending.slice();queue.unshift(request);pending=queue;
            retryDelay.interval=Math.min(1000,100*Math.pow(2,request.retries));retryDelay.restart();
            return;
        }
        current=null;
        completed(request,message === "",response,message);
        Qt.callLater(pump);
    }
    Timer { id:retryDelay; onTriggered:root.pump() }
    Process {
        id: process
        stdout: StdioCollector { id: output; waitForEnd:true }
        onExited: function(code,status) { root.finish(output.text,code); }
        onRunningChanged: if(!running) Qt.callLater(root.pump)
    }
    Timer {
        id: watchdog; interval:12000
        onTriggered: {
            process.running=false;
            root.finish('{"error":"Core timed out. Check current settings before retrying; the write may have completed."}',1);
        }
    }
}
