.pragma library
function span(name) { return name === "small" ? {columns:1,rows:1} : {columns:2,rows:name === "large" ? 2 : 1}; }
function geometry(name, g) {
    var s=span(name); g=g || {cell:192,gapX:0,gapY:0};
    return {width:s.columns*g.cell+(s.columns-1)*g.gapX,height:s.rows*g.cell+(s.rows-1)*g.gapY};
}
function cell(x,y,g) { return {column:Math.round((x-g.x)/(g.cell+g.gapX)),row:Math.round((y-g.y)/(g.cell+g.gapY))}; }
function target(x,y,size,monitor,workspace,g) {
    var c=cell(x,y,g), s=span(size);
    return {column:c.column,row:c.row,columns:s.columns,rows:s.rows,monitor:monitor,workspace:workspace};
}
function valid(t,g,blockers) {
    if(t.column<0 || t.row<0 || t.column+t.columns>g.columns || t.row+t.rows>g.rows) return false;
    return !blockers.some(function(b) {
        return t.monitor===b.monitor && (t.workspace==null || b.workspace==null || t.workspace===b.workspace)
            && t.column<b.column+b.columns && b.column<t.column+t.columns
            && t.row<b.row+b.rows && b.row<t.row+t.rows;
    });
}
function legacyName(name) { return {small:"compact",medium:"standard",large:"wide"}[name] || "standard"; }
