#!/usr/bin/env python3
"""Two real World Clock editors + production Core settings/queue + durable Rust CLI.
Qt adapts Process to QProcess. No Quickshell/Wayland or sandbox acceptance claim.
Usage: python3 tests/worldclock_integration.py BINARY WORLD_CLOCK_CHECKOUT
"""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time

os.environ.setdefault('QT_QPA_PLATFORM', 'offscreen')
from PySide6.QtCore import QObject, QUrl, QMetaObject, Q_ARG, Q_RETURN_ARG, Qt
from PySide6.QtGui import QGuiApplication
from PySide6.QtQml import QQmlApplicationEngine, qmlRegisterType
from PySide6.QtQuick import QQuickItem
from PySide6.QtTest import QTest

root = Path(__file__).resolve().parents[1]
binary = Path(sys.argv[1]).resolve()
clock = Path(sys.argv[2]).resolve()


from qt_process import Process


qmlRegisterType(Process, 'Quickshell.Io', 1, 0, 'Process')
app = QGuiApplication([])


def spin(predicate, message):
    deadline = time.monotonic() + 15
    while time.monotonic() < deadline:
        app.processEvents()
        if predicate():
            return
        QTest.qWait(10)
    raise AssertionError(message)


def call(obj, method, *args):
    return QMetaObject.invokeMethod(obj, method, Q_RETURN_ARG('QVariant'),
                                   *[Q_ARG('QVariant', value) for value in args])


with tempfile.TemporaryDirectory() as directory:
    temp = Path(directory)
    os.environ['XDG_DATA_HOME'] = str(temp / 'data')
    os.environ['XDG_STATE_HOME'] = str(temp / 'state')
    os.environ.pop('OMARCHY_WIDGET_BROKER', None)

    def cli(*args):
        return json.loads(subprocess.check_output([str(binary), *args], text=True))

    # Stage only production package files, as the widget's installer does.
    package = temp / 'package'
    package.mkdir()
    for name in ['widget.json', 'WorldClock.qml', 'Settings.qml', 'CityEditor.qml', 'Label.qml', 'Model.js', 'LICENSE']:
        shutil.copyfile(clock / name, package / name)
    shutil.copytree(clock / 'scripts', package / 'scripts')
    cli('install', str(package))
    first = 'io.github.tcballard.worldclock'
    cli('add', first)
    second = cli('duplicate', first)['updated']
    cli('workspace', second, '2')

    def placements():
        return {entry['instanceId']: entry['placement'] for entry in cli('list')['installed']}

    original = placements()
    for name in ['Commons', 'Ui']:
        shutil.copytree(root / name, temp / 'qs' / name)
    io = temp / 'Quickshell' / 'Io'
    io.mkdir(parents=True)
    (io / 'qmldir').write_text('module Quickshell.Io\nStdioCollector 1.0 StdioCollector.qml\n')
    (io / 'StdioCollector.qml').write_text('import QtQuick\nQtObject { property string text:""; property bool waitForEnd:false }')

    source = (root / 'Host.qml').read_text()
    # Exercise production controller functions, settingsContext, Loader and panel.
    # Only top-level Quickshell windows and the transport adapter are substituted.
    controller = source[source.index('Item {'):source.index('    function screenFor(')]
    controller=controller.replace('Quickshell.env("OMARCHY_WIDGET_REVEAL") === "1"', 'false')
    controller = controller.replace('id: root', 'id: root; objectName:"controller"; anchors.fill:parent', 1)
    controller = controller.replace('Quickshell.env("OMARCHY_WIDGET_ROLE") === "manager"', 'false')
    helper_line = next(line for line in controller.splitlines() if 'readonly property string helper:' in line)
    controller = controller.replace(helper_line, '    property string helper: ' + json.dumps(str(binary)))
    context = source[source.index('    QtObject {\n        id: settingsContext'):source.index('    FloatingWindow {')]
    panel = source[source.index('        Core.SettingsPanel {'):source.index('    Variants {')].rsplit('\n    }', 1)[0]
    harness = '''import QtQuick
import qs.Commons
import "''' + (root / 'qml').as_uri() + '''" as Core
Window {
    width:520; height:560; visible:true
''' + controller + context + panel + '''
    function loadSnapshot(text) { installed=JSON.parse(text).installed; return true; }
    function openEditor(id) { configure(id); return configuring; }
    function draft() { return JSON.stringify(settingsContext.draftSettings); }
    function closeEditor() { closeSettings(); return true; }
    function pending() { return operation.busy; }
    function current() { return configuring; }
    function activeEditor() { return settingsLoader.item; }
    function openGeneration() { return settingsGeneration; }
    function oldAcknowledgement(id,generation) {
        operation.completed({args:["save",id],token:{instance:id,generation:generation}},true,{},"");
        return configuring;
    }
    }
}
'''
    preview = temp / 'Integration.qml'
    preview.write_text(harness)
    engine = QQmlApplicationEngine()
    engine.addImportPath(str(temp))
    warnings = []
    engine.warnings.connect(lambda errors: warnings.extend(e.toString() for e in errors))
    engine.load(QUrl.fromLocalFile(str(preview)))
    assert engine.rootObjects(), '\n'.join(warnings)
    window = engine.rootObjects()[0]
    controller = window.findChild(QObject, 'controller')
    panel = window.findChild(QObject, 'settings-panel')
    call(controller, 'loadSnapshot', json.dumps(cli('list')))

    editor_roots = []

    def open_editor(instance):
        assert call(controller, 'openEditor', instance) == instance
        spin(lambda: panel.property('canSave'), 'Real World Clock editor did not load')
        QTest.qWait(50)
        item = call(controller, 'activeEditor')
        editor_roots.append(item)
        return item.findChild(QObject, 'city-editor')

    def button(name):
        QTest.qWait(50)
        item = window.findChild(QQuickItem, name)
        assert item and item.isEnabled(), name
        QMetaObject.invokeMethod(item, 'clicked')

    def draft():
        return json.loads(call(controller, 'draft'))

    # Cancel must discard an actual widget edit, with no write or cross-instance changes.
    editor = open_editor(first)
    QMetaObject.invokeMethod(editor, 'add', Q_ARG('QVariant', 'Europe/Paris'))
    assert draft()['cities'][-1]['zone'] == 'Europe/Paris'
    generation = call(controller, 'openGeneration')
    assert call(controller, 'openEditor', first) == first
    assert draft()['cities'][-1]['zone'] == 'Europe/Paris', 'Repeated gear press erased the draft'
    assert call(controller, 'openEditor', second) == first, 'Opening another clock erased the draft'
    button('cancel-button')
    assert placements() == original
    editor = open_editor(first)
    assert all(city['zone'] != 'Europe/Paris' for city in draft()['cities'])
    assert call(controller, 'oldAcknowledgement', first, generation) == first, 'Old save closed a new editor'
    spin(lambda: not call(controller, 'pending'), 'Acknowledgement refresh did not finish')
    QMetaObject.invokeMethod(editor, 'add', Q_ARG('QVariant', 'Europe/Paris'))
    button('save-button')
    spin(lambda: call(controller, 'current') == '', 'Save did not close after durable acknowledgement')
    spin(lambda: not call(controller, 'pending'), 'Save queue did not settle')
    after_first = placements()
    assert after_first[first]['settings']['cities'][-1]['zone'] == 'Europe/Paris'
    assert after_first[second] == original[second], 'Saving first clock changed its sibling'

    # Escape must reach Core from the embedded CityEditor, without writing anything.
    editor = open_editor(second)
    QMetaObject.invokeMethod(editor, 'add', Q_ARG('QVariant', 'Asia/Kathmandu'))
    QMetaObject.invokeMethod(editor, 'forceActiveFocus')
    QTest.keyClick(window, Qt.Key.Key_Escape)
    assert call(controller, 'current') == '', 'City editor swallowed Core Cancel'
    assert placements() == after_first

    # A concurrent external edit must retain the visible draft and reject stale Save.
    editor = open_editor(second)
    QMetaObject.invokeMethod(editor, 'add', Q_ARG('QVariant', 'Asia/Kathmandu'))
    concurrent = dict(original[second]['settings'], displayMode='analogue')
    cli('save', second, json.dumps({'revision': original[second]['revision'], 'settings': concurrent}))
    assert call(controller, 'openEditor', first) == second
    button('save-button')
    spin(lambda: 'changed elsewhere' in panel.property('error'), 'Stale Save was not rejected')
    assert draft()['cities'][-1]['zone'] == 'Asia/Kathmandu'
    assert placements()[second]['settings'] == concurrent
    button('cancel-button')
    spin(lambda: not call(controller, 'pending'), 'Conflict refresh did not settle')
    editor = open_editor(second)
    assert draft()['displayMode'] == 'analogue'
    assert not panel.property('error'), 'Reopened editor retained a previous save error'
    QMetaObject.invokeMethod(editor, 'add', Q_ARG('QVariant', 'Asia/Kathmandu'))
    button('save-button')
    spin(lambda: call(controller, 'current') == '', 'Second clock save did not finish')
    spin(lambda: not call(controller, 'pending'), 'Second clock queue did not settle')
    saved = placements()
    assert saved[first] == after_first[first]
    assert saved[second]['settings']['cities'][-1]['zone'] == 'Asia/Kathmandu'
    assert saved[second]['settings']['displayMode'] == 'analogue'
    for instance in [first, second]:
        for key in ['packageId', 'definitionId', 'monitor', 'cell', 'size', 'workspace']:
            assert saved[instance].get(key) == original[instance].get(key), key
    cli('hide', first)
    assert placements()[second] == saved[second]
    cli('add', first)
    reopened = placements()  # Every CLI invocation is a fresh process reopening durable state.
    assert reopened[first]['settings'] == saved[first]['settings']
    assert reopened[second] == saved[second]
    open_editor(first)
    cli('remove-instance', first)
    QMetaObject.invokeMethod(controller, 'refresh')
    spin(lambda: call(controller, 'current') == '', 'Removing an instance left its editor open')
    spin(lambda: not call(controller, 'pending'), 'Removal refresh did not settle')
    assert placements()[second] == saved[second]
    assert not warnings, '\n'.join(warnings)
    print('PASS: two World Clocks; independent Save/Cancel/Escape; repeated open; old acknowledgement; stale conflict; hide/show and fresh-process reopen')
