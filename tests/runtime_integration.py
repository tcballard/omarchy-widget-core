#!/usr/bin/env python3
"""Real Rust CLI + production QML controller/queue/settings surface.
Only Quickshell Process is adapted to Qt QProcess. No Wayland claims.
"""
import ast, json, os, subprocess, sys, tempfile, time
from pathlib import Path
os.environ.setdefault('QT_QPA_PLATFORM', 'offscreen')
from PySide6.QtCore import QObject, Property, Signal, QProcess, QUrl, QPoint, QMetaObject, Qt
from PySide6.QtQml import QQmlApplicationEngine, qmlRegisterType
from PySide6.QtGui import QGuiApplication
from PySide6.QtQuick import QQuickItem, QQuickWindow
from PySide6.QtTest import QTest
root = Path(__file__).resolve().parents[1]
binary = Path(sys.argv[1]).resolve()
commands = []
class Process(QObject):
    exited = Signal(int, int)
    changed = Signal()
    def __init__(self, parent=None):
        super().__init__(parent)
        self.cmd, self.out, self.run = [], None, False
        self.child = QProcess(self)
        self.child.finished.connect(self.finish)
        self.child.errorOccurred.connect(self.failed)
    command = Property('QVariantList', lambda s:s.cmd, lambda s,v:setattr(s,'cmd',v), notify=changed)
    stdout = Property(QObject, lambda s:s.out, lambda s,v:setattr(s,'out',v), notify=changed)
    def start(self, value):
        if value:
            self.run = True
            self.changed.emit()
            commands.append(list(self.cmd))
            self.child.start(self.cmd[0], self.cmd[1:])
        else:
            self.child.kill()
    running = Property(bool, lambda s:s.run, start, notify=changed)
    def finish(self, code, status):
        self.run = False
        self.out.setProperty('text', bytes(self.child.readAllStandardOutput()).decode())
        self.exited.emit(code, 0)
        self.changed.emit()
    def failed(self, error):
        if error == QProcess.ProcessError.FailedToStart:
            self.run = False
            self.changed.emit()
qmlRegisterType(Process, 'Quickshell.Io', 1, 0, 'Process')
app = QGuiApplication([])
def spin(predicate, message):
    deadline = time.monotonic()+15
    while time.monotonic()<deadline:
        app.processEvents()
        if predicate(): return
        QTest.qWait(10)
    raise AssertionError(message)
with tempfile.TemporaryDirectory() as directory:
    temp=Path(directory)
    os.environ['XDG_DATA_HOME']=str(temp/'data')
    os.environ['XDG_STATE_HOME']=str(temp/'state')
    def cli(*args):
        return json.loads(subprocess.check_output([str(binary),*args],text=True))
    fixture=temp/'fixture'; fixture.mkdir()
    (fixture/'View.qml').write_text('import QtQuick\nItem {}')
    (fixture/'widget.json').write_text(json.dumps(dict(schemaVersion=2,kind='desktop-widget',coreApi=2,id='io.example.fixture',name='Fixture',version='0.0.2',entryPoint='View.qml',families=['medium','large'],defaultFamily='large',defaults={'cities':[]})))
    cli('install',str(fixture)); cli('add','io.example.fixture')
    # Extract fixtures without importing/executing the separate smoke-test script.
    tree=ast.parse((root/'tests/qml_smoke.py').read_text())
    stubs=next(ast.literal_eval(n.value) for n in tree.body if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='stubs' for t in n.targets))
    for directory in ['Commons','Ui']:
        for file in (root/directory).iterdir(): stubs['qs/'+directory+'/'+file.name]=file.read_text()
    stubs['Quickshell/Io/qmldir']='module Quickshell.Io\nStdioCollector 1.0 StdioCollector.qml\n'
    stubs['Quickshell/Io/StdioCollector.qml']='import QtQuick\nQtObject { property string text:""; property bool waitForEnd:false }'
    for name,content in stubs.items():
        path=temp/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_text(content)
    service=(root/'Host.qml').read_text().split('    function screenFor(',1)[0]
    body=service[service.index('Item {'):].replace('id: root','id: controller; objectName:"controller"',1)
    # Production expressions refer to root; keep its id and use controller objectName.
    body=body.replace('Quickshell.env("OMARCHY_WIDGET_REVEAL") === "1"', 'false')
    body=body.replace('id: controller;','id: root;')
    # The harness supplies the runner role; the real Quickshell environment is not present.
    body=body.replace('Quickshell.env("OMARCHY_WIDGET_ROLE") === "manager"', 'false')
    line=next(x for x in body.splitlines() if 'readonly property string helper:' in x)
    body=body.replace(line,'    property string helper: '+json.dumps(str(binary)))
    snapshot=cli('list')
    body+='''
        function initialize(value) { installed=JSON.parse(value).installed; }
        function backlog() {
            for(var i=1;i<=20;i++) execute(["place","io.example.fixture",JSON.stringify({column:i,row:0,monitor:"DP-1",size:"large"})]);
            refresh();
        }
        function staleSave() { save("io.example.fixture",{cities:[{label:"Stale",zone:"UTC"}]},0); }
    }
'''
    preview=temp/'Integration.qml'
    preview.write_text('''import QtQuick
import qs.Commons
import "'''+(root/'qml').as_uri()+'''" as Core
import "'''+(root/'qml/Grid.js').as_uri()+'''" as Grid
Window {
    id:host; width:520; height:560; visible:true
'''+body+'''
    Core.SettingsPanel {
        id:panel; objectName:"panel"; anchors.fill:parent
        property var state: root.saveStates["io.example.fixture"] || ({saving:false,error:"",saved:false})
        property bool acknowledged:false
        busy:state.saving; error:state.error
        onSaveRequested: root.save("io.example.fixture",{cities:[{label:"Paris",zone:"Europe/Paris"}]},0)
        onStateChanged: if(state.saved) acknowledged=true
        Core.Label { anchors.centerIn:parent; text:"Paris · Europe/Paris" }
    }
    function initialize(value) { root.initialize(value); }
    function backlog() { root.backlog(); }
    function staleSave() { root.staleSave(); }
    function gridCheck() {
        var g={x:10,y:42,cell:192,gapX:10,gapY:10,columns:4,rows:3};
        return Grid.geometry("medium",g).width===394 && Grid.cell(212,42,g).column===1
            && Grid.valid(Grid.target(212,42,"medium","DP-1",null,g),g,[]);
    }
}
''')
    engine=QQmlApplicationEngine();engine.addImportPath(str(temp))
    warnings=[];engine.warnings.connect(lambda errors:warnings.extend(e.toString() for e in errors))
    engine.load(QUrl.fromLocalFile(str(preview)))
    assert engine.rootObjects(), '\n'.join(warnings)
    window=engine.rootObjects()[0]
    from PySide6.QtCore import Q_ARG, Q_RETURN_ARG
    QMetaObject.invokeMethod(window,'initialize',Q_ARG('QVariant',json.dumps(snapshot)))
    assert QMetaObject.invokeMethod(window,'gridCheck',Q_RETURN_ARG('QVariant'))
    QMetaObject.invokeMethod(window,'backlog')
    button=window.findChild(QQuickItem,'save-button')
    assert button
    app.processEvents()
    position=button.mapToScene(QPoint(int(button.width()/2),int(button.height()/2)))
    QTest.mouseClick(window,Qt.MouseButton.LeftButton,Qt.KeyboardModifier.NoModifier,QPoint(int(position.x()),int(position.y())))
    panel=window.findChild(QObject,'panel')
    spin(lambda:panel.property('acknowledged'),'Save never received a durable acknowledgement')
    spin(lambda:len(commands)>=3,'Queue stalled')
    saved=cli('list')['installed'][0]['placement']
    assert saved['settings']['cities'][0]['label']=='Paris'
    assert saved['revision']==1
    assert saved['cell']=={'column':0,'row':0}, 'Unavailable desktop must reject placement and retain preference'
    assert len([c for c in commands if 'place' in c])==1, 'Placement requests were not coalesced'
    QMetaObject.invokeMethod(window,'staleSave')
    spin(lambda:bool(panel.property('error')),'Stale editor was silently accepted')
    assert 'changed elsewhere' in panel.property('error')
    assert cli('list')['installed'][0]['placement']['settings']['cities'][0]['label']=='Paris'
    assert not warnings, '\n'.join(warnings)
    # A killed writer must not strand the registry lock.
    lock=temp/'state/omarchy/widgets/registry.lock'
    holder=subprocess.Popen([sys.executable,'-c','import fcntl,sys,time; f=open(sys.argv[1],"r+"); fcntl.flock(f,fcntl.LOCK_EX); print("locked",flush=True); time.sleep(30)',str(lock)],stdout=subprocess.PIPE,text=True)
    assert holder.stdout.readline().strip()=='locked'
    holder.kill();holder.wait()
    cli('hide','io.example.fixture')
    print('PASS: real QML Save → queued Rust operation → durable disk → reopen; stale revision rejected; placement coalescing; crash lock release')
