import QtQuick
import QtQuick.Layouts
import QtQuick.Effects
import qs.Commons
import qs.Ui as Ui

FocusScope {
    id: root
    property string title: ""
    property bool editing: false
    property string sizeName: "standard"
    property string monitorName: ""
    property string notice: ""
    property real moveStepX: 192
    property real moveStepY: 192
    property bool invalidTarget: false
    property var appearance: ({})
    function number(key, fallback, min, max) {
        var v=appearance[key];
        return typeof v === "number" && isFinite(v) ? Math.max(min,Math.min(max,v)) : fallback;
    }
    readonly property string labelFamily: typeof appearance.fontFamily === "string" ? appearance.fontFamily : Style.font.family
    default property alias contents: slot.data
    property bool configurable: false
    signal configureRequested()
    signal editRequested()
    signal sizeRequested()
    signal monitorRequested()
    signal hideRequested()
    signal moved(real dx, real dy)
    signal finishedMoving()
    signal escapeRequested()
    // Mask the entire card, including widget content, to the same rounded edge.
    property Item roundedMask: Rectangle {
        parent:root
        width: root.width; height: root.height
        radius: root.number("radius",Math.max(Style.cornerRadius,Style.space(12)),0,1000)
        color: "white"; visible:false; layer.enabled:true
    }
    layer.enabled: GraphicsInfo.api !== GraphicsInfo.Software
    layer.effect: MultiEffect { maskEnabled:true; maskSource:root.roundedMask }
    activeFocusOnTab: true
    Keys.onPressed: function(event) {
        if(event.key===Qt.Key_Escape){root.escapeRequested();event.accepted=true;return;}
        if(!root.editing)return;
        if(event.key===Qt.Key_Left)root.moved(-root.moveStepX,0);
        else if(event.key===Qt.Key_Right)root.moved(root.moveStepX,0);
        else if(event.key===Qt.Key_Up)root.moved(0,-root.moveStepY);
        else if(event.key===Qt.Key_Down)root.moved(0,root.moveStepY);
        else return;
        root.finishedMoving();event.accepted=true;
    }
    Rectangle {
        objectName:"widget-surface"
        anchors.fill:parent
        color:Qt.rgba(Color.background.r,Color.background.g,Color.background.b,root.number("backgroundAlpha",1,0.6,1))
        border.width:root.editing?1:root.number("borderWidth",1,0,100)
        border.color:root.editing?(root.invalidTarget?Color.urgent:Color.accent):Qt.rgba(Color.foreground.r,Color.foreground.g,Color.foreground.b,root.number("borderAlpha",0.12,0,1))
        radius:root.number("radius",Math.max(Style.cornerRadius,Style.space(12)),0,1000)
    }
    ColumnLayout {
        anchors.fill:parent;spacing:0
        Item {
            visible:root.editing || root.configurable || root.appearance.showTitle === true
            Layout.fillWidth:true;Layout.preferredHeight:visible?Style.space(42):0
            MouseArea {
                id: drag;anchors.fill:parent;enabled:root.editing
                cursorShape:pressed?Qt.ClosedHandCursor:Qt.OpenHandCursor
                property point lastPoint:Qt.point(0,0)
                onPressed:function(mouse){root.forceActiveFocus();lastPoint=mapToGlobal(mouse.x,mouse.y)}
                onPositionChanged:function(mouse){if(!pressed)return;var point=mapToGlobal(mouse.x,mouse.y);root.moved(point.x-lastPoint.x,point.y-lastPoint.y);lastPoint=point}
                onReleased:root.finishedMoving()
                onCanceled:root.finishedMoving()
            }
            RowLayout {
                anchors.fill:parent;anchors.leftMargin:Style.space(14);anchors.rightMargin:Style.space(7);spacing:Style.space(5)
                Label { text:root.editing?"⠿  "+root.title:root.appearance.showTitle === true ? root.title : "";font.family:root.labelFamily;Layout.fillWidth:true }
                Ui.Button { objectName:"settings-gear"; text:"⚙"; visible:root.configurable; focusable:true; Accessible.name:root.title+" settings"; onClicked:root.configureRequested() }
                Ui.Button { visible:root.editing || root.appearance.showTitle === true; objectName:"edit-button";text:root.editing?"Done":"Arrange";fontSize:Style.font.bodySmall;focusable:true;onClicked:root.editRequested() }
            }
        }
        Rectangle { visible:root.editing;Layout.fillWidth:true;Layout.preferredHeight:visible?1:0;color:Color.foreground;opacity:0.1 }
        RowLayout {
            visible:root.editing;Layout.fillWidth:true;Layout.margins:Style.space(4);spacing:0
            Ui.Button { text:root.sizeName;focusable:true;onClicked:root.sizeRequested();Layout.fillWidth:true }
            Ui.Button { text:"↗";tooltipText:root.monitorName;focusable:true;onClicked:root.monitorRequested() }
            Ui.Button { text:"Hide";focusable:true;onClicked:root.hideRequested() }
        }
        Item {id:slot;Layout.fillWidth:true;Layout.fillHeight:true;Layout.margins:Math.max(Style.space(root.number("padding",12,8,24)),Math.ceil(root.number("radius",Math.max(Style.cornerRadius,Style.space(12)),0,1000)*0.3));clip:true}
        Label { visible:root.notice!=="";text:root.notice;color:Color.urgent;font.pixelSize:Style.font.bodySmall;Layout.fillWidth:true;Layout.margins:Style.space(10) }
    }
}
