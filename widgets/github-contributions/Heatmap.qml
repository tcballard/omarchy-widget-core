import QtQuick
import "Model.js" as Model

FocusScope {
    id: root
    required property var weeks
    required property var theme
    required property var metrics
    property bool green: true
    property bool compact: false
    property int selected: -1
    signal inspected(var day)
    readonly property real pitch: width / Math.max(1,weeks.length)
    readonly property real cell: Math.max(1,Math.min(compact ? 9 : 13,pitch-2))
    readonly property real rowPitch: cell+2
    implicitHeight: 16+7*rowPitch
    activeFocusOnTab: true
    Accessible.role: Accessible.Chart
    Accessible.name: "Contribution calendar. Use arrow keys to inspect days."
    Accessible.description: selected>=0 && weeks.length ? Model.describe(weeks[Math.floor(selected/7)][selected%7]) : ""
    function select(index, direction) {
        var count=weeks.length*7;
        index=Math.max(0,Math.min(count-1,index));
        while(index>=0 && index<count && !weeks[Math.floor(index/7)][index%7]) index+=direction;
        if(index>=0 && index<count) {selected=index;inspected(weeks[Math.floor(index/7)][index%7]);}
    }
    Keys.onPressed: function(event) {
        var delta=event.key===Qt.Key_Left ? -7 : event.key===Qt.Key_Right ? 7 : event.key===Qt.Key_Up ? -1 : event.key===Qt.Key_Down ? 1 : 0;
        if(delta) {select(selected<0 ? 0 : selected+delta,delta<0 ? -1 : 1);event.accepted=true;}
    }
    function colorFor(level) {
        if(level===0) return Qt.rgba(theme.foreground.r,theme.foreground.g,theme.foreground.b,0.09);
        if(green) return ["transparent","#0e4429","#006d32","#26a641","#39d353"][level];
        return Qt.rgba(theme.accent.r,theme.accent.g,theme.accent.b,[0,0.25,0.45,0.7,1][level]);
    }
    Repeater {
        model:Model.months(root.weeks)
        Text {
            required property var modelData
            x:modelData.column*root.pitch;y:0
            text:modelData.label;textFormat:Text.PlainText
            color:root.theme.muted;font.family:root.metrics.font.family;font.pixelSize:9
        }
    }
    Repeater {
        model:root.weeks.length*7
        Rectangle {
            required property int index
            readonly property var day:root.weeks[Math.floor(index/7)][index%7]
            x:Math.floor(index/7)*root.pitch;y:16+(index%7)*root.rowPitch
            width:root.cell;height:root.cell;radius:Math.min(2,width/4)
            visible:day!==null
            color:day ? root.colorFor(day.level) : "transparent"
            border.width:root.activeFocus && root.selected===index ? 1 : 0
            border.color:root.theme.foreground
            MouseArea {
                anchors.fill:parent;hoverEnabled:true
                onEntered:root.inspected(parent.day)
                onExited:if(!root.activeFocus)root.inspected(null)
                onClicked:{root.forceActiveFocus();root.select(parent.index,1);}
            }
        }
    }
}
