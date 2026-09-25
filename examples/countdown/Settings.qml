import QtQuick
Column {
    id:root
    required property var settingsContext
    spacing:12
    Text {text:"Label";color:root.settingsContext.theme.foreground}
    TextInput {width:parent.width;text:root.settingsContext.draftSettings.label;color:root.settingsContext.theme.foreground;maximumLength:80;Accessible.name:"Countdown label";onTextEdited:root.settingsContext.draftSettings=Object.assign({},root.settingsContext.draftSettings,{label:text})}
    Text {text:"Duration in seconds (next Start)";color:root.settingsContext.theme.foreground}
    TextInput {width:parent.width;text:String(root.settingsContext.draftSettings.durationSeconds);color:root.settingsContext.theme.foreground;validator:IntValidator{bottom:1;top:86400}
        Accessible.name:"Duration in seconds";onTextEdited:if(acceptableInput)root.settingsContext.draftSettings=Object.assign({},root.settingsContext.draftSettings,{durationSeconds:Number(text)})}
}
