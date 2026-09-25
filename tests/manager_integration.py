#!/usr/bin/env python3
"""Production manager/controller/queue -> real registry. Offscreen Qt, not Hyprland."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time

os.environ.setdefault('QT_QPA_PLATFORM', 'offscreen')
from PySide6.QtCore import QObject, QUrl, QMetaObject, Q_ARG, Q_RETURN_ARG, QPoint, Qt
from PySide6.QtGui import QGuiApplication, QImage, QColor
from PySide6.QtQml import QQmlApplicationEngine, qmlRegisterType
from PySide6.QtQuick import QQuickWindow
from PySide6.QtTest import QTest
from qt_process import Process

root = Path(__file__).resolve().parents[1]
binary = Path(sys.argv[1]).resolve()
qmlRegisterType(Process, 'Quickshell.Io', 1, 0, 'Process')
app = QGuiApplication([])


def call(obj, method, *args):
    return QMetaObject.invokeMethod(obj, method, Q_RETURN_ARG('QVariant'),
                                   *[Q_ARG('QVariant', value) for value in args])


def spin(predicate, message):
    deadline = time.monotonic() + 15
    while time.monotonic() < deadline:
        app.processEvents()
        if predicate():
            return
        QTest.qWait(10)
    raise AssertionError(message)


with tempfile.TemporaryDirectory() as directory:
    temp = Path(directory)
    os.environ['XDG_DATA_HOME'] = str(temp / 'data')
    os.environ['XDG_STATE_HOME'] = str(temp / 'state')
    os.environ.pop('OMARCHY_WIDGET_BROKER', None)

    def cli(*args):
        return json.loads(subprocess.check_output([str(binary), *args], text=True))

    package = temp / 'fixture'
    shutil.copytree(root / 'examples/notes', package)
    manifest = json.loads((package / 'widget.json').read_text())
    package_id = manifest['id']
    # A real bounded PNG tests the static-art path; other families test the footprint fallback.
    preview = QImage(192, 192, QImage.Format.Format_RGB32)
    preview.fill(QColor('#b3cb92'))
    assert preview.save(str(package / 'preview.png'))
    manifest['previews'] = {'small': 'preview.png'}
    (package / 'widget.json').write_text(json.dumps(manifest))
    cli('install', str(package))
    for name in ['Commons', 'Ui']:
        shutil.copytree(root / name, temp / 'qs' / name)
    io = temp / 'Quickshell' / 'Io'
    io.mkdir(parents=True)
    (io / 'qmldir').write_text('module Quickshell.Io\nStdioCollector 1.0 StdioCollector.qml\n')
    (io / 'StdioCollector.qml').write_text('import QtQuick\nQtObject { property string text:""; property bool waitForEnd:false }')
    source = (root / 'Host.qml').read_text()
    controller = source[source.index('Item {'):source.index('    function screenFor(')]
    controller=controller.replace('Quickshell.env("OMARCHY_WIDGET_REVEAL") === "1"', 'false')
    controller = controller.replace('id: root', 'id: root; objectName:"controller"; anchors.fill:parent', 1)
    controller = controller.replace('Quickshell.env("OMARCHY_WIDGET_ROLE") === "manager"', 'true')
    helper = next(line for line in controller.splitlines() if 'readonly property string helper:' in line)
    controller = controller.replace(helper, '    property string helper: ' + json.dumps(str(binary)))
    manager = source[source.index('Core.Manager {'):source.index('        // manager-content-end')]
    manager = manager.replace('Core.Manager {', 'Core.Manager {\n id:manager;objectName:"manager";', 1)
    harness = '''import QtQuick
import qs.Commons
import "''' + (root / 'qml').as_uri() + '''" as Core
Window {
 width:760;height:660;visible:true
''' + controller + manager + '''
 function sync() { return refresh(); }
 function pending() { return operation.busy; }
 function snapshot() { return JSON.stringify({catalog:catalog,installed:installed,retained:retained}); }
 function chooseView(value) { manager.view=value; return true; }
 }
}
'''
    path = temp / 'Integration.qml'
    path.write_text('import "'+(root/'qml/Declarative.js').as_uri()+'" as Declarative\n'+harness)
    engine = QQmlApplicationEngine()
    engine.addImportPath(str(temp))
    warnings = []
    engine.warnings.connect(lambda errors: warnings.extend(e.toString() for e in errors))
    engine.load(QUrl.fromLocalFile(str(path)))
    assert engine.rootObjects(), '\n'.join(warnings)
    window = engine.rootObjects()[0]
    controller = window.findChild(QObject, 'controller')
    manager = window.findChild(QObject, 'manager')

    def sync():
        call(controller, 'sync')
        spin(lambda: not call(controller, 'pending'), 'Manager queue did not settle')
        QTest.qWait(80)

    def state():
        return json.loads(call(controller, 'snapshot'))

    def find(item, name):
        if item.objectName() == name:
            return item
        for child in item.childItems():
            found = find(child, name)
            if found is not None:
                return found
        return None

    def click(name):
        QTest.qWait(50)
        item = find(window.contentItem(), name)
        assert item is not None and item.isVisible() and item.isEnabled(), name
        # Scroll the control itself, not just its potentially taller delegate.
        for list_name in ['instances-list', 'available-list']:
            listing = find(window.contentItem(), list_name)
            ancestor = item.parentItem()
            while ancestor is not None and ancestor != listing:
                ancestor = ancestor.parentItem()
            if ancestor == listing:
                call(listing, 'ensureVisible', item)
                QTest.qWait(30)
                local = item.mapToItem(listing, QPoint(0, 0))
                assert local.y() >= -1 and local.y()+item.height() <= listing.height()+1, (name, local.y(), listing.height())
        point = item.mapToScene(QPoint(int(item.width()/2), int(item.height()/2)))
        assert 0 <= point.y() < window.height(), (name, point.y())
        QTest.mouseClick(window, Qt.MouseButton.LeftButton, Qt.KeyboardModifier.NoModifier,
                         QPoint(int(point.x()), int(point.y())))
        sync()

    def show_instance(instance):
        click('instances-tab')
        # ListView delegates are virtualised; scroll the requested row into view.
        entries = [e for e in state()['installed'] if e.get('placement')] + state()['retained']
        index = next(i for i, e in enumerate(entries) if e['instanceId'] == instance)
        listing = find(window.contentItem(), 'instances-list')
        QMetaObject.invokeMethod(listing, 'positionViewAtIndex', Q_ARG('int', index), Q_ARG('int', 0))
        QTest.qWait(80)

    def placement(instance):
        return next(e['placement'] for e in cli('list')['installed'] if e['instanceId'] == instance)

    sync()
    assert len(state()['catalog']) == 1 and state()['catalog'][0]['instanceCount'] == 0
    click('available-tab')
    click('family-' + package_id + '-small')
    click('add-' + package_id)
    first = state()['installed'][0]['instanceId']
    assert placement(first)['size'] == 'small'
    cli('configure', first, json.dumps({'note': 'First instance'}))
    sync()
    # Add is fresh defaults; it does not reuse or clone the first instance.
    click('family-' + package_id + '-large')
    click('add-' + package_id)
    second = next(e['instanceId'] for e in state()['installed'] if e['instanceId'] != first)
    assert placement(second)['size'] == 'large'
    assert placement(second)['settings'] == manifest['defaults']
    assert len(state()['catalog']) == 1 and state()['catalog'][0]['instanceCount'] == 2
    show_instance(first)
    click('duplicate-' + first)
    third = next(e['instanceId'] for e in state()['installed'] if e['instanceId'] not in [first, second])
    assert placement(third)['settings'] == placement(first)['settings']
    show_instance(first)
    before_second = placement(second)
    click('toggle-' + first)
    assert not placement(first)['enabled'] and placement(second) == before_second
    click('toggle-' + first)
    assert placement(first)['enabled']
    click('configure-' + first)
    assert cli('list')['runtime']['edit']['instance'] == first
    # Cancel and Escape do not remove anything; confirmation then removes only first.
    click('remove-' + first)
    click('cancel-removal')
    assert placement(first)['enabled']
    click('remove-' + first)
    QTest.keyClick(window, Qt.Key.Key_Escape)
    sync()
    assert placement(first)['enabled']
    click('remove-' + first)
    click('confirm-remove')
    assert len([e for e in state()['installed'] if e.get('placement')]) == 2
    assert placement(second) == before_second and cli('list')['runtime']['edit'] is None
    click('available-tab')
    click('uninstall-' + package_id)
    click('uninstall-keep')
    assert not state()['catalog'] and not state()['installed'] and len(state()['retained']) == 2
    for retained in state()['retained']:
        assert not retained['placement']['enabled']
    show_instance(third)
    click('remove-' + third)
    click('confirm-remove')
    assert len(state()['retained']) == 1
    cli('install', str(package))
    sync()
    assert not state()['retained'] and not placement(second)['enabled']
    assert placement(second)['settings'] == before_second['settings']
    show_instance(second)
    click('toggle-' + second)
    assert placement(second)['enabled']
    # Tab through the taller instance row. Focus must scroll into the viewport;
    # activate Hide with Space and Show with Return, without mouse repositioning.
    show_instance(second)
    button = find(window.contentItem(), 'configure-' + second)
    button.forceActiveFocus()
    for _ in range(12):
        if window.activeFocusItem().objectName() == 'toggle-' + second:
            break
        QTest.keyClick(window, Qt.Key.Key_Tab)
        QTest.qWait(30)
    assert window.activeFocusItem().objectName() == 'toggle-' + second
    focused=window.activeFocusItem(); listing=find(window.contentItem(),'instances-list')
    local=focused.mapToItem(listing,QPoint(0,0))
    assert local.y()>=-1 and local.y()+focused.height()<=listing.height()+1
    QTest.keyClick(window,Qt.Key.Key_Space);sync();assert not placement(second)['enabled']
    # Polling updates can replace delegates; re-find the same control by identity.
    find(window.contentItem(),'toggle-'+second).forceActiveFocus()
    QTest.keyClick(window,Qt.Key.Key_Return);sync();assert placement(second)['enabled']
    out = root / 'test-results'
    out.mkdir(exist_ok=True)
    assert window.grabWindow().save(str(out / 'manager-instances.png'))
    click('available-tab')
    click('family-' + package_id + '-small')
    assert window.grabWindow().save(str(out / 'manager-available.png'))
    click('uninstall-' + package_id)
    click('uninstall-delete')
    assert not state()['catalog'] and not state()['retained'] and not state()['installed']
    cli('install', str(package))
    sync()
    assert state()['catalog'][0]['instanceCount'] == 0
    assert not warnings, '\n'.join(warnings)
    print('PASS: real manager mouse actions -> Rust registry; selected sizes, fresh Add, Duplicate, Configure, Hide/Show, removal confirmation/Cancel/Escape, uninstall keep/delete, retained removal and reinstall')
