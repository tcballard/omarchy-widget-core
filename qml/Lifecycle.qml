import QtQuick
QtObject {
    property bool active:false
    readonly property bool backgroundAllowed:active
    readonly property string lifecycle:active ? "visible" : "suspended"
    signal becameVisible()
    signal becameHidden()
    signal suspending()
    signal resuming()
    onActiveChanged: {
        if(active) {resuming();becameVisible();}
        else {becameHidden();suspending();}
    }
}
