import QtQuick
import qs.Commons
Text {
    property var result: ({state:"loading",data:null,error:""})
    text: result.error || (result.state==="loading" ? "Loading…" : result.state==="stale" ? "Showing last known data" : result.state==="unavailable" ? "Service unavailable" : "")
    visible:text.length>0
    color:Color.muted
    font.family:Style.font.family
    font.pixelSize:Style.font.bodySmall
    textFormat:Text.PlainText
    wrapMode:Text.Wrap
}
