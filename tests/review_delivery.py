#!/usr/bin/env python3
"""Production controller/queue recovery under refused closes and one-shot errors.

Qt substitutes transport/windows; content-report replies are controlled here.
The actual scoped dispatcher and Unix transport are covered by Rust tests/CI.
"""
import fcntl
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time

os.environ.setdefault('QT_QPA_PLATFORM', 'offscreen')
from PySide6.QtCore import QObject, QUrl, QMetaObject, Q_ARG, Q_RETURN_ARG
from PySide6.QtGui import QGuiApplication
from PySide6.QtQml import QQmlApplicationEngine, qmlRegisterType
from PySide6.QtTest import QTest
from qt_process import Process

root = Path(__file__).resolve().parents[1]
binary = Path(sys.argv[1]).resolve()
qmlRegisterType(Process, 'Quickshell.Io', 1, 0, 'Process')
app = QGuiApplication([])


def call(obj, method, *args):
    return QMetaObject.invokeMethod(obj, method, Q_RETURN_ARG('QVariant'),
                                   *[Q_ARG('QVariant', v) for v in args])


def spin(predicate, message):
    until = time.monotonic() + 15
    while time.monotonic() < until:
        app.processEvents()
        if predicate():
            return
        QTest.qWait(10)
    raise AssertionError(message)


with tempfile.TemporaryDirectory() as tmp:
    temp = Path(tmp)
    os.environ.update(XDG_DATA_HOME=str(temp/'data'), XDG_STATE_HOME=str(temp/'state'))
    os.environ.pop('OMARCHY_WIDGET_BROKER', None)

    def cli(*args):
        return json.loads(subprocess.check_output([str(binary), *map(str, args)], text=True))

    package = temp/'package'
    cli('new-qml', package, 'io.review.delivery', 'Delivery')
    manifest = json.loads((package/'widget.json').read_text())
    (package/'Settings.qml').rename(package/'Set#tings.qml')
    manifest['settingsEntryPoint'] = 'Set#tings.qml'
    (package/'widget.json').write_text(json.dumps(manifest))
    cli('install', package)
    instance = cli('create', manifest['id'], 'small')['updated']
    attempts = temp/'attempts'
    shim = temp/'helper'
    shim.write_text('#!'+sys.executable+'\n'+f'''
import json, pathlib, subprocess, sys
if sys.argv[1] == 'content-failed':
    path=pathlib.Path({str(attempts)!r})
    count=int(path.read_text())+1 if path.exists() else 1
    path.write_text(str(count))
    if count<=5:
        print(json.dumps(dict(error='Registry busy; retry after the current operation finishes',code='busy')))
        sys.exit(1)
    print('true')
else:
    sys.exit(subprocess.call([{str(binary)!r},*sys.argv[1:]]))
''')
    shim.chmod(0o700)
    for name in ['Commons', 'Ui']:
        shutil.copytree(root/name, temp/'qs'/name)
    io = temp/'Quickshell/Io'
    io.mkdir(parents=True)
    (io/'qmldir').write_text('module Quickshell.Io\nStdioCollector 1.0 StdioCollector.qml\n')
    (io/'StdioCollector.qml').write_text('import QtQuick\nQtObject {property string text:"";property bool waitForEnd:false}')
    source = (root/'Host.qml').read_text()
    controller = source[source.index('Item {'):source.index('    function screenFor(')]
    controller = controller.replace('id: root', 'id: root;objectName:"controller"', 1)
    for role in ['ROLE', 'REVEAL']:
        controller = controller.replace('Quickshell.env("OMARCHY_WIDGET_'+role+'") === "'+('manager' if role=='ROLE' else '1')+'"', 'false')
    helper_line = next(line for line in controller.splitlines() if 'readonly property string helper:' in line)
    controller = controller.replace(helper_line, 'property string helper:'+json.dumps(str(shim)))
    context = source[source.index('    QtObject {\n        id: settingsContext'):source.index('    FloatingWindow {\n        title: "Widget settings"')]
    panel = source[source.index('        Core.SettingsPanel {'):source.index('    Variants {')].rsplit('\n    }', 1)[0]
    (temp/'Broken.qml').write_text('import QtQuick\nItem { broken syntax }')
    harness = 'import QtQuick\nimport qs.Commons\nimport "'+(root/'qml').as_uri()+'" as Core\nWindow {width:520;height:560;visible:true\n'+controller+context+panel+'''
    Loader { id:broken; onStatusChanged:if(status===Loader.Error)root.contentFailed(INSTANCE) }
    function openEditor() { configure(INSTANCE); return true; }
    function closeEditor() { closeSettings(); return true; }
    function current() { return configuring; }
    function closing() { return pendingClose!==null; }
    function pending() { return operation.busy; }
    function ready() { return settingsLoader.status===Loader.Ready; }
    function failedReports() { return Object.keys(contentFailures).length; }
    function failOnce() { broken.source=BROKEN; return true; }
    function fullQueueClose() {
        var queue=[];for(var i=0;i<64;i++)queue.push({args:["control","refresh"],token:""});
        operation.pending=queue;closeSettings();operation.pump();return pendingClose!==null;
    }
    Component.onCompleted:refresh()
}}
'''.replace('INSTANCE', json.dumps(instance)).replace('BROKEN', json.dumps((temp/'Broken.qml').as_uri()))
    file = temp/'Test.qml'
    file.write_text('import "'+(root/'qml/Declarative.js').as_uri()+'" as Declarative\n'+harness)
    engine = QQmlApplicationEngine()
    engine.addImportPath(str(temp))
    warnings = []
    engine.warnings.connect(lambda errors: warnings.extend(e.toString() for e in errors))
    engine.load(QUrl.fromLocalFile(str(file)))
    assert engine.rootObjects(), warnings
    window = engine.rootObjects()[0]
    c = window.findChild(QObject, 'controller')
    spin(lambda: not call(c, 'pending'), 'Initial snapshot did not settle')
    call(c, 'openEditor')
    spin(lambda: call(c, 'current') == instance and call(c, 'ready'), 'Encoded editor path failed')
    serial = cli('list')['runtime']['edit']['serial']
    with (temp/'state/omarchy/widgets/registry.lock').open('r+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        call(c, 'closeEditor')
        QTest.qWait(2600)
        assert call(c, 'current') == '' and call(c, 'closing')
        assert cli('list')['runtime']['edit']['serial'] == serial
        fcntl.flock(lock, fcntl.LOCK_UN)
    spin(lambda: not call(c, 'closing') and not call(c, 'pending'), 'Close was lost after contention')
    assert cli('list')['runtime']['edit'] is None
    call(c, 'openEditor')
    spin(lambda: call(c, 'ready'), 'Editor did not reopen')
    cli('edit-done', instance, serial)
    assert cli('list')['runtime']['edit']['serial'] != serial
    assert call(c, 'fullQueueClose'), 'Queue-full close was discarded'
    spin(lambda: not call(c, 'closing') and not call(c, 'pending'), 'Queue-full close never recovered')
    assert cli('list')['runtime']['edit'] is None
    call(c, 'failOnce')
    spin(lambda: attempts.exists() and int(attempts.read_text()) >= 6 and call(c, 'failedReports') == 0,
         'One-shot Loader failure was not retained until acknowledged')
    assert warnings and all('Broken.qml' in warning for warning in warnings), warnings
    print('PASS: encoded editor URL; busy and full-queue close acknowledgement; stale close safety; one-shot content failure retried until acknowledged')
