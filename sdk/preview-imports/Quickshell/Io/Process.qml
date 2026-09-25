import QtQuick
// Capture-only interface: never starts a process or runs widget commands.
QtObject {
    property var command: []
    property bool running: false
    property bool stdinEnabled: false
    property var stdout: null
    property var stderr: null
    signal exited(int exitCode, int exitStatus)
    signal started()
    function write(text) {}
    function closeStdin() {}
}
