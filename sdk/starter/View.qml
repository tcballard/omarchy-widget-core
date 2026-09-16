import QtQuick
Item {
    id:root
    required property var widgetContext
    readonly property var theme:widgetContext.theme
    readonly property var metrics:widgetContext.metrics
    Column {
        anchors.fill:parent; spacing:root.metrics.space(12)
        Text {
            width:parent.width; text:root.widgetContext.settings.title
            color:root.theme.foreground; font.family:root.metrics.font.family
            font.pixelSize:root.metrics.font.title; font.bold:true; textFormat:Text.PlainText
            elide:Text.ElideRight
        }
        Text {
            width:parent.width
            text:root.widgetContext.family==="small" ? root.widgetContext.settings.note.split("\n")[0] : root.widgetContext.settings.note
            color:root.theme.foreground; font.family:root.metrics.font.family
            font.pixelSize:root.metrics.font.body; textFormat:Text.PlainText
            wrapMode:Text.Wrap; maximumLineCount:root.widgetContext.family==="small" ? 3 : root.widgetContext.family==="medium" ? 4 : 12
            elide:Text.ElideRight
        }
        Text {
            visible:root.widgetContext.family==="large"; width:parent.width
            text:root.widgetContext.settings.note.length+" characters · saved revision "+root.widgetContext.settingsRevision
            color:root.theme.muted; font.family:root.metrics.font.family
            font.pixelSize:root.metrics.font.bodySmall; textFormat:Text.PlainText
        }
    }
}
