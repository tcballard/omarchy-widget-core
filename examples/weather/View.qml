import QtQuick
import qs.Commons
Item {
    id: root
    required property var widgetContext
    readonly property var result: widgetContext.weather
    function refresh() { widgetContext.requestWeather(widgetContext.settings.latitude,widgetContext.settings.longitude); }
    Component.onCompleted: refresh()
    Connections { target:root.widgetContext; function onBecameVisible() { root.refresh(); } function onSettingsChanged() { root.refresh(); } }
    Timer { interval:10000; running:root.widgetContext.active; repeat:true; onTriggered:root.refresh() }
    Column {
        anchors.centerIn:parent; width:parent.width; spacing:8
        Text { width:parent.width; text:root.widgetContext.settings.label; color:Color.foreground; font.family:Style.font.family; font.pixelSize:Style.font.title; elide:Text.ElideRight; horizontalAlignment:Text.AlignHCenter }
        Text { width:parent.width; text:root.result.data ? Math.round(root.result.data.temperatureC)+"°C" : "—"; color:Color.foreground; font.pixelSize:32; horizontalAlignment:Text.AlignHCenter }
        Text { width:parent.width; text:root.result.error || (root.result.state==="loading" ? "Loading weather…" : root.result.state==="stale" ? "Last known weather" : "Open-Meteo"); color:Color.muted; wrapMode:Text.Wrap; horizontalAlignment:Text.AlignHCenter; font.pixelSize:10 }
        Text { visible:root.widgetContext.family!=="small" && !!root.result.data; width:parent.width; text:root.result.data ? "Observed "+new Date(root.result.data.observedAt*1000).toLocaleTimeString() : ""; color:Color.muted; horizontalAlignment:Text.AlignHCenter }
        Text { visible:root.widgetContext.family==="large"; width:parent.width; text:"Coordinates: "+root.widgetContext.settings.latitude+", "+root.widgetContext.settings.longitude+"\nForecast data: Open-Meteo.com"; color:Color.muted; wrapMode:Text.Wrap; horizontalAlignment:Text.AlignHCenter }
    }
}
