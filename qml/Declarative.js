.pragma library
function isDeclarative(entry) { return !!entry && entry.manifest.renderer === "declarative"; }
function enabled(entry,catalog) {
    for(var i=0;i<catalog.length;i++)if(catalog[i].packageId===entry.packageId)return !catalog[i].packageDisabled;
    return false;
}
function wanted(entries,catalog,edit,shown) {
    return entries.some(function(e){return isDeclarative(e) && enabled(e,catalog) && ((shown && e.placement && e.placement.enabled) || e.instanceId===edit);});
}
function resolve(binding,settings,item) {
    if(typeof binding==="string")return binding;
    if(!binding)return "";
    return binding.setting!==undefined?settings[binding.setting]:item[binding.item];
}
function zones(entries,catalog,shown,desktop) {
    var result={};
    if(!shown)return [];
    function visit(node,settings,item,depth,family) {
        if(!node || depth>6)return;
        if(node.type==="clock") {var zone=resolve(node.timezone,settings,item);if(typeof zone==="string")result[zone]=true;}
        else if(node.type==="repeat") {
            var values=resolve(node.items,settings,item) || [];
            if(family==="small" && settings.homeZone) {var home=values.filter(function(x){return x.zone===settings.homeZone;});if(home.length)values=home;}
            values.slice(0,family==="small"?1:(family==="medium"?3:6)).forEach(function(x){visit(node.child,settings,x,depth+1,family);});
        } else (node.children || []).forEach(function(x){visit(x,settings,item,depth+1,family);});
    }
    entries.forEach(function(e){
        if(!isDeclarative(e) || !enabled(e,catalog) || !e.placement || !e.placement.enabled || !e.effective)return;
        var p=e.placement;
        if(p.workspace!==null && p.workspace!==undefined && desktop.monitors[e.effective.monitor]!==p.workspace)return;
        visit(e.manifest.view,p.settings,{},0,p.size);
    });
    return Object.keys(result).sort().slice(0,128);
}
