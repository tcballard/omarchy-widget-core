import QtQuick

Item {
    required property var settingsContext
    TextEdit {
        anchors.fill:parent; focus:true; selectByMouse:true; wrapMode:TextEdit.Wrap
        color:settingsContext.theme.foreground
        font.family:settingsContext.metrics.font.family
        font.pixelSize:settingsContext.metrics.font.body
        text:settingsContext.draftSettings.note || ""
        onTextEdited:settingsContext.draftSettings=Object.assign({},settingsContext.draftSettings,{note:text})
    }
}
