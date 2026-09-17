#!/usr/bin/env python3
"""A non-starter consumer exercises the published preview contract and imports."""
import json
import subprocess
import sys
import tempfile
from pathlib import Path

root = Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory() as tmp:
    package = Path(tmp)/'package'
    package.mkdir()
    (package/'widget.json').write_text(json.dumps(dict(
        schemaVersion=2, coreApi=3, kind='desktop-widget', id='io.review.preview',
        name='Contract probe', version='0.0.1', entryPoint='View.qml',
        families=['small', 'medium', 'large'], defaultFamily='small', defaults={})))
    (package/'View.qml').write_text('''import QtQuick
import Quickshell.Io
Item {
    required property var widgetContext
    readonly property string identity:widgetContext.instanceId+widgetContext.packageId+widgetContext.definitionId
    readonly property int api:widgetContext.api
    readonly property int revision:widgetContext.settingsRevision
    readonly property string family:widgetContext.family
    readonly property bool active:widgetContext.active
    readonly property bool backgroundAllowed:widgetContext.backgroundAllowed
    readonly property string lifecycle:widgetContext.lifecycle
    readonly property var theme:widgetContext.theme
    readonly property var metrics:widgetContext.metrics
    readonly property var appearance:widgetContext.appearance
    readonly property var settings:widgetContext.settings
    readonly property var weather:widgetContext.weather
    readonly property var state:widgetContext.saveState
    readonly property bool saving:widgetContext.saving
    readonly property bool saved:widgetContext.saved
    readonly property string error:widgetContext.saveError
    Connections { target:widgetContext; function onBecameVisible() {} function onBecameHidden() {} function onSuspending() {} function onResuming() {} }
    Process { command:["/usr/bin/touch","SIDE_EFFECT"];running:true;stdout:StdioCollector {waitForEnd:true} }
    IpcHandler {target:"preview-inert"}
    Text {anchors.centerIn:parent;text:parent.family}
    Component.onCompleted: {
        if(widgetContext.requestWeather(0,0)!==false || widgetContext.saveSettings({},0)!==false) throw new Error("Preview allowed a side effect");
        widgetContext.requestConfigure();
    }
}'''.replace('SIDE_EFFECT', str(Path(tmp)/'unexpected-command')))
    def capture_command(output):
        if '--sandbox' in sys.argv:
            return ['bash', str(root/'sdk/capture-package'), str(package), str(output)]
        return [sys.executable, str(root/'sdk/render_previews.py'), str(output), '--package', str(package)]
    subprocess.run(capture_command(Path(tmp)/'previews'), check=True)
    assert not (Path(tmp)/'unexpected-command').exists()
    assert len(list((Path(tmp)/'previews').glob('*.png'))) == 3
    manifest = json.loads((package/'widget.json').read_text())
    manifest.update(schemaVersion=1, coreApi=1)
    (package/'widget.json').write_text(json.dumps(manifest))
    rejected = subprocess.run(capture_command(Path(tmp)/'rejected'), text=True, capture_output=True)
    assert rejected.returncode != 0 and 'API 1 has been removed' in rejected.stderr, rejected
    assert not list((Path(tmp)/'rejected').glob('*.png'))
print('PASS: public identity/state/lifecycle preview API; inert process and IPC imports; no save/network/process side effects; API 1 rejected')
