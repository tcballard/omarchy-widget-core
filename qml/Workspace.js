function visible(assignment, monitor, desktop) {
    if (assignment === undefined || assignment === null) return true;
    return !!desktop && desktop.available === true && !!desktop.monitors
        && desktop.monitors[monitor] === assignment;
}
if (typeof module !== "undefined") module.exports={visible:visible};
