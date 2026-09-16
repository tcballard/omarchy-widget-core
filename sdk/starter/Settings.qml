import QtQuick
import QtQuick.Layouts
Item {
    id:root
    required property var settingsContext
    readonly property string validationError:title.text.trim().length ? "" : "Enter a title."
    function updateDraft() {settingsContext.draftSettings={title:title.text,note:note.text};}
    ColumnLayout {
        anchors.fill:parent; spacing:12
        Text {text:"Title"; color:root.settingsContext.theme.foreground}
        TextInput {
            id:title; Layout.fillWidth:true; color:root.settingsContext.theme.foreground
            font.family:root.settingsContext.metrics.font.family
            text:root.settingsContext.draftSettings.title; maximumLength:80; selectByMouse:true
            Accessible.name:"Title"; onTextEdited:root.updateDraft()
        }
        Text {text:"Note"; color:root.settingsContext.theme.foreground}
        TextEdit {
            id:note; Layout.fillWidth:true; Layout.fillHeight:true; color:root.settingsContext.theme.foreground
            font.family:root.settingsContext.metrics.font.family
            text:root.settingsContext.draftSettings.note; textFormat:TextEdit.PlainText; selectByMouse:true; wrapMode:TextEdit.Wrap
            Accessible.name:"Note"; onTextChanged:if(activeFocus)root.updateDraft()
        }
    }
}
