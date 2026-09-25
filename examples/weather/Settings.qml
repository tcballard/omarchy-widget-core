import QtQuick
import QtQuick.Layouts
import qs.Commons
Item {
    required property var settingsContext
    implicitHeight:200
    readonly property string validationError: !label.text.trim() ? "Enter a place name." : !latitude.acceptableInput || !longitude.acceptableInput ? "Latitude must be −90 to 90; longitude −180 to 180." : ""
    function updateDraft() { settingsContext.draftSettings={label:label.text.trim(),latitude:Number(latitude.text),longitude:Number(longitude.text)}; }
    ColumnLayout {
        anchors.fill:parent
        Text { text:"Place name, latitude and longitude"; color:Color.foreground }
        TextInput { id:label; Layout.fillWidth:true; text:settingsContext.draftSettings.label; color:Color.foreground; maximumLength:80; onTextEdited:updateDraft(); Accessible.name:"Place name" }
        TextInput { id:latitude; Layout.fillWidth:true; text:String(settingsContext.draftSettings.latitude); color:Color.foreground; onTextEdited:updateDraft(); validator:DoubleValidator {bottom:-90;top:90;locale:"C"} Accessible.name:"Latitude" }
        TextInput { id:longitude; Layout.fillWidth:true; text:String(settingsContext.draftSettings.longitude); color:Color.foreground; onTextEdited:updateDraft(); validator:DoubleValidator {bottom:-180;top:180;locale:"C"} Accessible.name:"Longitude" }
    }
}
