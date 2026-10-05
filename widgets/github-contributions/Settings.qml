import QtQuick
import QtQuick.Layouts
import "Model.js" as Model

Item {
    id:root
    required property var settingsContext
    readonly property var theme:settingsContext.theme
    readonly property var metrics:settingsContext.metrics
    readonly property string validationError:Model.validUsername(settingsContext.draftSettings.username) ? "" : "Enter a valid GitHub username."
    implicitHeight:220
    function set(key,value) {
        var draft=Object.assign({},settingsContext.draftSettings);draft[key]=value;settingsContext.draftSettings=draft;
    }
    ColumnLayout {
        anchors.fill:parent;spacing:12
        Text { text:"GitHub username";color:root.theme.foreground;font.family:root.metrics.font.family;font.pixelSize:14 }
        Rectangle {
            Layout.fillWidth:true;implicitHeight:38;color:"transparent";radius:6
            border.color:username.activeFocus ? root.theme.accent : root.theme.muted
            TextInput {
                id:username;anchors.fill:parent;anchors.margins:8
                text:root.settingsContext.draftSettings.username;maximumLength:39;selectByMouse:true
                color:root.theme.foreground;font.family:root.metrics.font.family;font.pixelSize:14
                onTextEdited:root.set("username",text.trim())
                Accessible.name:"GitHub username";activeFocusOnTab:true
            }
        }
        RowLayout {
            Repeater {
                model:["GitHub green","Theme accent"]
                Rectangle {
                    id:choice
                    required property string modelData
                    Layout.fillWidth:true;implicitHeight:36;radius:6
                    color:root.settingsContext.draftSettings.palette===modelData ? Qt.rgba(root.theme.accent.r,root.theme.accent.g,root.theme.accent.b,0.15) : "transparent"
                    border.width:activeFocus ? 2 : 1;border.color:activeFocus ? root.theme.accent : root.theme.muted
                    activeFocusOnTab:true;Accessible.role:Accessible.RadioButton;Accessible.name:modelData
                    Accessible.checked:root.settingsContext.draftSettings.palette===modelData
                    function choose() {root.set("palette",modelData);}
                    Keys.onSpacePressed:choose();Keys.onReturnPressed:choose()
                    Text {anchors.centerIn:parent;text:choice.modelData;color:root.theme.foreground;font.family:root.metrics.font.family;font.pixelSize:12}
                    MouseArea {anchors.fill:parent;onClicked:{parent.forceActiveFocus();parent.choose();}}
                }
            }
        }
        Text { Layout.fillWidth:true;text:"Uses your public profile. No token required. GitHub may omit private activity; totals can differ from your signed-in graph.";wrapMode:Text.Wrap;color:root.theme.muted;font.family:root.metrics.font.family;font.pixelSize:12 }
        TextEdit { Layout.fillWidth:true;text:Model.validUsername(root.settingsContext.draftSettings.username) ? "https://github.com/"+root.settingsContext.draftSettings.username : "";textFormat:TextEdit.PlainText;readOnly:true;selectByMouse:true;color:root.theme.accent;font.family:root.metrics.font.family;font.pixelSize:12;Accessible.name:"Public profile URL (select to copy)" }
        Item {Layout.fillHeight:true}
    }
}
