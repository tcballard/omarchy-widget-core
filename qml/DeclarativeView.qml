import QtQuick
DeclarativeNode {
    property var widgetContext
    property var definition: ({})
    property var clockTimes: ({})
    node:definition.view || ({})
    settings:widgetContext ? widgetContext.settings : ({})
    family:widgetContext ? widgetContext.family : "medium"
    times:clockTimes
    clip:true
}
