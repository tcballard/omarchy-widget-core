#!/usr/bin/env python3
"""Exercise the production idle-exit Timer in real Quickshell, without a desktop."""
import os
from pathlib import Path
import re
import subprocess
import tempfile

root = Path(__file__).resolve().parents[1]
source = (root / 'Host.qml').read_text()
timer = re.search(r'    Timer \{\n        interval:250\n.*?\n    \}', source, re.S)
assert timer, 'Production idle-exit timer not found'

with tempfile.TemporaryDirectory(prefix='widget-qs-exit-') as directory:
    path = Path(directory)
    (path / 'runtime').mkdir(mode=0o700)
    (path / 'shell.qml').write_text('''import QtQuick
import Quickshell
Item {
    id: root
    property bool managerRole: true
    property bool declarativeWanted: false
    property string configuring: ""
    property bool snapshotReady: true
    property bool managerOpen: false
    property bool editing: false
    property bool revealing: false
    property var pendingClose: null
    property var contentFailures: ({})
    QtObject { id: operation; property bool busy: false }
''' + timer.group() + '\n}\n')
    environment = dict(os.environ, QT_QPA_PLATFORM='offscreen',
                       QT_QUICK_BACKEND='software', QT_QPA_PLATFORMTHEME='basic',
                       XDG_RUNTIME_DIR=str(path / 'runtime'))
    result = subprocess.run(['qs', '--no-color', '-p', str(path)], env=environment,
                            stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                            text=True, timeout=10)
    assert result.returncode == 0, result.stdout
    assert 'TypeError' not in result.stdout and 'ERROR' not in result.stdout, result.stdout
print('PASS: production idle-exit timer terminates real Quickshell cleanly')
