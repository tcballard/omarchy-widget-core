pragma Singleton
import QtQuick

// Core-owned compatibility facade, independent of the Omarchy shell engine.
QtObject {
    id: root
    property color foreground: "#e4e8df"
    property color background: "#171c1a"
    property color accent: "#b3cb92"
    property color muted: "#8a9588"
    property color urgent: "#e5a085"
    function apply(palette) {
        foreground=palette.foreground || "#e4e8df";
        background=palette.background || "#171c1a";
        accent=palette.accent || "#b3cb92";
        urgent=palette.red || "#e5a085";
        muted=Qt.rgba(foreground.r,foreground.g,foreground.b,0.65);
    }
}
