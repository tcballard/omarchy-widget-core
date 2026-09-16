"""Qt transport adapter for production QML integration tests; no Wayland emulation."""
from PySide6.QtCore import QObject, Property, Signal, QProcess


class Process(QObject):
    exited = Signal(int, int)
    changed = Signal()

    def __init__(self, parent=None):
        super().__init__(parent)
        self.cmd, self.out, self.run = [], None, False
        self.child = QProcess(self)
        self.child.finished.connect(self.finish)
        self.child.errorOccurred.connect(self.failed)

    command = Property('QVariantList', lambda s: s.cmd, lambda s, v: setattr(s, 'cmd', v), notify=changed)
    stdout = Property(QObject, lambda s: s.out, lambda s, v: setattr(s, 'out', v), notify=changed)

    def start(self, value):
        if value:
            self.run = True
            self.changed.emit()
            self.child.start(self.cmd[0], self.cmd[1:])
        else:
            self.child.kill()

    running = Property(bool, lambda s: s.run, start, notify=changed)

    def finish(self, code, status):
        self.run = False
        self.out.setProperty('text', bytes(self.child.readAllStandardOutput()).decode())
        self.exited.emit(code, 0)
        self.changed.emit()

    def failed(self, error):
        if error == QProcess.ProcessError.FailedToStart:
            self.run = False
            self.changed.emit()


