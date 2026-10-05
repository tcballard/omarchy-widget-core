import QtQuick
import QtQuick.Layouts
import "Model.js" as Model

Item {
    id: root
    required property var widgetContext
    readonly property var theme: widgetContext.theme
    readonly property var metrics: widgetContext.metrics
    readonly property var result: widgetContext.github || ({state:"unavailable",data:null,error:"Update Widget Core to use GitHub Contributions."})
    readonly property var calendar: result.data
    readonly property bool small: widgetContext.family === "small"
    readonly property bool large: widgetContext.family === "large"
    property string inspected: ""
    function refresh() { if(widgetContext.active && widgetContext.requestGithub)widgetContext.requestGithub(widgetContext.settings.username); }
    Component.onCompleted:refresh()
    Connections {
        target:root.widgetContext
        function onBecameVisible() { root.refresh(); }
        function onResuming() { root.refresh(); }
        function onSettingsChanged() { root.inspected="";root.refresh(); }
    }
    // Poll bounded broker state; the provider TTL is 15 minutes, shared across instances.
    Timer { interval:10000;running:root.widgetContext.active;repeat:true;onTriggered:root.refresh() }
    ColumnLayout {
        anchors.fill:parent;spacing:root.small ? 2 : root.large ? 12 : 5
        Text {
            Layout.fillWidth:true;Layout.rightMargin:22
            text:"@"+root.widgetContext.settings.username;textFormat:Text.PlainText
            color:root.theme.muted;font.family:root.metrics.font.family;font.pixelSize:12
            elide:Text.ElideRight
        }
        RowLayout {
            Layout.fillWidth:true;spacing:8
            Text {
                text:root.calendar ? Model.formatCount(root.calendar.total) : "—";textFormat:Text.PlainText
                color:root.theme.foreground;font.family:root.metrics.font.family;font.pixelSize:root.large ? 36 : 28;font.weight:Font.DemiBold
                Accessible.name:text+" contributions in the last year"
            }
            Text {
                visible:!root.small;Layout.fillWidth:true
                text:"contributions\nin the last year";textFormat:Text.PlainText
                color:root.theme.muted;font.family:root.metrics.font.family;font.pixelSize:11
            }
        }
        Repeater {
            model:root.calendar ? Model.sections(root.calendar.days,root.widgetContext.family) : []
            Heatmap {
                required property var modelData
                Layout.fillWidth:true;Layout.preferredHeight:implicitHeight
                weeks:modelData;theme:root.theme;metrics:root.metrics;compact:root.small
                green:root.widgetContext.settings.palette!=="Theme accent"
                onInspected:function(day) { root.inspected=Model.describe(day); }
            }
        }
        Text {
            visible:!root.calendar;Layout.fillWidth:true;Layout.fillHeight:true
            text:root.result.state==="loading" ? "Loading contributions…" : "Contributions unavailable\nAllow GitHub access in Widgets → Add widgets, or retry when online."
            textFormat:Text.PlainText;wrapMode:Text.Wrap;verticalAlignment:Text.AlignVCenter
            color:root.theme.muted;font.family:root.metrics.font.family;font.pixelSize:12
            Accessible.description:root.result.error || ""
        }
        Item { visible:!!root.calendar;Layout.fillHeight:true;Layout.minimumHeight:0 }
        Text {
            Layout.fillWidth:true
            text:root.inspected || (root.result.state==="stale" ? "Last known data · retrying" : root.small ? "13 weeks · year total" : "GitHub · public contributions")
            textFormat:Text.PlainText;color:root.result.state==="stale" ? root.theme.urgent : root.theme.muted
            font.family:root.metrics.font.family;font.pixelSize:root.small ? 9 : 10
            elide:Text.ElideRight
        }
    }
}
