import QtQuick
import QtQuick.Layouts
import qs.Ui as Ui
Item {
    id:root
    required property var widgetContext
    property real now:Date.now()
    readonly property var state:widgetContext.settings
    readonly property bool running:state.endsAt>0
    readonly property int remaining:running ? Math.max(0,Math.ceil((state.endsAt-now)/1000)) : state.remainingSeconds
    function start() { return widgetContext.saveSettings(Object.assign({},state,{endsAt:Date.now()+state.durationSeconds*1000,remainingSeconds:state.durationSeconds}),widgetContext.settingsRevision); }
    function pause() { return widgetContext.saveSettings(Object.assign({},state,{endsAt:0,remainingSeconds:Math.max(0,Math.ceil((state.endsAt-Date.now())/1000))}),widgetContext.settingsRevision); }
    function resume() { return widgetContext.saveSettings(Object.assign({},state,{endsAt:Date.now()+state.remainingSeconds*1000}),widgetContext.settingsRevision); }
    Timer { interval:250;repeat:true;running:root.widgetContext.active;onTriggered:root.now=Date.now() }
    Connections {target:root.widgetContext;function onActiveChanged(){root.now=Date.now();}}
    ColumnLayout {
        anchors.fill:parent;spacing:4
        Text {text:root.state.label;color:root.widgetContext.theme.foreground;font.family:root.widgetContext.metrics.font.family;elide:Text.ElideRight;Layout.fillWidth:true}
        Text {text:Math.floor(root.remaining/60)+":"+String(root.remaining%60).padStart(2,"0");color:root.widgetContext.theme.foreground;font.pixelSize:root.widgetContext.family==="large"?64:30;Layout.fillWidth:true}
        Flow {
            Layout.fillWidth:true;spacing:4
            Ui.Button {text:root.running?"Pause":root.state.remainingSeconds<root.state.durationSeconds?"Resume":"Start";enabled:!root.widgetContext.saving;onClicked:root.running?root.pause():root.state.remainingSeconds<root.state.durationSeconds?root.resume():root.start()}
        }
        Text {text:root.widgetContext.saveError;visible:text!=="";color:root.widgetContext.theme.urgent;wrapMode:Text.Wrap;Layout.fillWidth:true}
    }
}
