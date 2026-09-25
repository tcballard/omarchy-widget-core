import QtQuick
import qs.Commons

// Only this Core-owned file is loaded recursively. Package strings are data.
Item {
    id: root
    property var node: ({})
    property var settings: ({})
    property var itemData: ({})
    property var times: ({})
    property string family: "medium"
    function value(binding) {
        if(typeof binding === "string") return binding;
        if(!binding) return "";
        if(binding.setting !== undefined) return settings[binding.setting];
        if(binding.item !== undefined) return itemData[binding.item];
        return "";
    }
    readonly property var childrenData: {
        if(node.type === "repeat") {
            var items=value(node.items) || [];
            var limit=family==="small"?1:(family==="medium"?3:6);
            if(family==="small" && settings.homeZone) {
                var home=items.filter(function(item){return item.zone===root.settings.homeZone;});
                if(home.length) return home.slice(0,1);
            }
            return items.slice(0,limit);
        }
        return node.children || [];
    }
    readonly property int columns: node.type==="column"?1:(node.type==="repeat" && family==="large"?2:Math.max(1,childrenData.length))
    readonly property int rows: Math.max(1,Math.ceil(childrenData.length/columns))
    Repeater {
        model: root.childrenData
        Loader {
            required property int index
            required property var modelData
            x:(index%root.columns)*root.width/root.columns
            y:Math.floor(index/root.columns)*root.height/root.rows
            width:Math.max(0,root.width/root.columns-4)
            height:Math.max(0,root.height/root.rows-4)
            source: "DeclarativeNode.qml"
            onLoaded: {
                item.node=Qt.binding(function(){return root.node.type==="repeat"?root.node.child:modelData;});
                item.itemData=Qt.binding(function(){return root.node.type==="repeat"?modelData:root.itemData;});
                item.settings=Qt.binding(function(){return root.settings;});
                item.times=Qt.binding(function(){return root.times;});
                item.family=Qt.binding(function(){return root.family;});
            }
        }
    }
    Text {
        anchors.fill:parent
        visible:root.node.type==="text"
        text:String(root.value(root.node.value) || "")
        textFormat:Text.PlainText
        color:Color.foreground
        font.family:Style.font.family
        font.pixelSize:root.node.style==="heading"?22:(root.node.style==="caption"?12:16)
        horizontalAlignment:Text.AlignHCenter
        verticalAlignment:Text.AlignVCenter
        elide:Text.ElideRight
    }
    readonly property var clock: times[value(node.timezone)] || null
    readonly property bool analogue: node.type==="clock" && value(node.mode)==="analogue"
    Text {
        anchors.fill:parent
        visible:root.node.type==="clock" && (!root.analogue || !root.clock)
        text:root.clock ? root.clock.time : "—"
        textFormat:Text.PlainText
        color:Color.foreground
        font.family:Style.font.family
        font.pixelSize:Math.min(30,width/3.5)
        horizontalAlignment:Text.AlignHCenter
        verticalAlignment:Text.AlignVCenter
    }
    Canvas {
        id:dial
        anchors.centerIn:parent
        width:Math.max(0,Math.min(parent.width,parent.height)-4);height:width
        visible:root.analogue && !!root.clock
        property var reading:root.clock
        property color ink:Color.foreground
        property color accent:Color.accent
        onReadingChanged:requestPaint()
        onInkChanged:requestPaint()
        onAccentChanged:requestPaint()
        onWidthChanged:requestPaint()
        onVisibleChanged:if(visible) requestPaint()
        onPaint: {
            var c=getContext("2d");c.clearRect(0,0,width,height);
            if(!reading || width<=0)return;
            var r=width/2-2;c.save();c.translate(width/2,height/2);
            c.strokeStyle=ink;c.lineWidth=1;c.beginPath();c.arc(0,0,r,0,Math.PI*2);c.stroke();
            for(var i=0;i<12;i++) { var a=i*Math.PI/6;c.beginPath();c.moveTo(Math.sin(a)*r*.85,-Math.cos(a)*r*.85);c.lineTo(Math.sin(a)*r*.95,-Math.cos(a)*r*.95);c.stroke(); }
            function hand(angle,length,thickness) {c.lineWidth=thickness;c.beginPath();c.moveTo(0,0);c.lineTo(Math.sin(angle)*r*length,-Math.cos(angle)*r*length);c.stroke();}
            hand((reading.hour%12+reading.minute/60)*Math.PI/6,.52,3);
            c.strokeStyle=accent;hand(reading.minute*Math.PI/30,.76,2);c.restore();
        }
    }
}
