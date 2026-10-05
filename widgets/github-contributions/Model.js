function validUsername(value) {
    return typeof value === "string" && value.length <= 39 && /^[a-zA-Z0-9]+(-[a-zA-Z0-9]+)*$/.test(value);
}
function weeks(days) {
    if (!days || !days.length) return [];
    var result = [], week = [null,null,null,null,null,null,null];
    days.forEach(function(day) {
        if (day.weekday === 0 && week.some(function(d) { return d !== null; })) {
            result.push(week); week = [null,null,null,null,null,null,null];
        }
        week[day.weekday] = day;
    });
    if (week.some(function(d) { return d !== null; })) result.push(week);
    return result;
}
function sections(days, family) {
    var all = weeks(days);
    if (family === "small") return [all.slice(-13)];
    if (family === "large") {
        var split = Math.ceil(all.length / 2);
        return [all.slice(0,split), all.slice(split)];
    }
    return [all];
}
function months(weeks) {
    var result = [], last = "";
    weeks.forEach(function(week, i) {
        var day = week.filter(function(d) { return d !== null; })[0];
        if (!day) return;
        var month = day.date.slice(0,7);
        if (month !== last) {
            result.push({column:i,label:["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"][Number(day.date.slice(5,7))-1]});
            last = month;
        }
    });
    // Do not overlap the final month label at the edge.
    return result.filter(function(m) { return m.column < weeks.length - 2; });
}
function formatCount(value) { return String(value).replace(/\B(?=(\d{3})+(?!\d))/g, ","); }
function describe(day) {
    return day ? formatCount(day.count) + (day.count === 1 ? " contribution · " : " contributions · ") + day.date : "";
}
if (typeof module !== "undefined") module.exports = {validUsername:validUsername,weeks:weeks,sections:sections,months:months,describe:describe};
