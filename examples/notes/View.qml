import QtQuick

Item {
    required property var widgetContext
    Text {
        anchors.fill:parent; text:widgetContext.settings.note || "Add a note"
        color:widgetContext.theme.foreground
        font.family:widgetContext.metrics.font.family
        font.pixelSize:widgetContext.metrics.font.body
        wrapMode:Text.Wrap; textFormat:Text.PlainText
    }
    TapHandler { onTapped:widgetContext.requestConfigure() }
}
