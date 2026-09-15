pragma Singleton
import QtQuick

QtObject {
    id: root
    property real scale: 1
    property int cornerRadius: 16
    property string fontFamily: "sans-serif"
    function spaceReal(value) { return value*scale; }
    function space(value) { return Math.round(value*scale); }
    property QtObject font: QtObject {
        property string family: root.fontFamily
        property real body: 12*root.scale
        property real bodySmall: 10*root.scale
        property real title: 16*root.scale
        property real heading: 18*root.scale
    }
    function apply(appearance) {
        scale=typeof appearance.scale === "number" ? Math.max(0.75,Math.min(2,appearance.scale)) : 1;
        fontFamily=typeof appearance.fontFamily === "string" && appearance.fontFamily.length ? appearance.fontFamily : "sans-serif";
        cornerRadius=typeof appearance.radius === "number" ? Math.max(0,Math.min(40,appearance.radius)) : 16;
    }
}
