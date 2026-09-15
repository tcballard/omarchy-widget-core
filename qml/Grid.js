.pragma library
var step = 16;
var inset = 16;
var families = {small:{width:192,height:192},medium:{width:400,height:192},large:{width:400,height:400}};
function snap(value, extent, length) {
    var maximum = Math.max(0, Math.floor((extent-length-inset)/step)*step);
    return Math.max(Math.min(inset,maximum), Math.min(maximum,Math.round(value/step)*step));
}
function geometry(name) { return families[name] || families.medium; }
function legacyName(name) { return {small:"compact",medium:"standard",large:"wide"}[name] || "standard"; }
