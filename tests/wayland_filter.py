#!/usr/bin/env python3
"""Exercise the real pinned proxy against an adversarial wire client and fake compositor.
No claim about visual/compositor integration: this tests actual protocol enforcement.
"""
import json, os, socket, struct, subprocess, sys, tempfile, time
from pathlib import Path
proxy, helper = map(lambda p: str(Path(p).resolve()), sys.argv[1:])
root = Path(__file__).resolve().parents[1]
def uint(n): return struct.pack('=I', n)
def string(s):
    data=s.encode()+b'\0';return uint(len(data))+data+b'\0'*((-len(data))%4)
def message(obj,opcode,payload=b''): return uint(obj)+uint(((len(payload)+8)<<16)|opcode)+payload
def recv(sock):
    def exact(n):
        data=b''
        while len(data)<n:
            part=sock.recv(n-len(data))
            if not part: raise EOFError()
            data+=part
        return data
    header=exact(8);size=struct.unpack('=II',header)[1]>>16
    return header+exact(size-8)
with tempfile.TemporaryDirectory(prefix='wl-filter-test-') as tmp:
    base=Path(tmp);up=base/'up';down=base/'down';listener=socket.socket(socket.AF_UNIX);listener.bind(str(up));listener.listen();listener.settimeout(3)
    hook=base/'policy';hook.write_text('#!/bin/sh\nexec '+"'"+helper.replace("'","'\\''")+"'"+' wayland-policy "$@"\n');hook.chmod(0o755)
    config=base/'config.toml';config.write_text('[socket]\nlisten='+json.dumps(str(down))+'\nupstream='+json.dumps(str(up))+'\n[exec]\nask_cmd='+json.dumps(str(hook))+'\n'+(root/'wayland-filter.toml').read_text())
    process=subprocess.Popen([proxy,str(config)],env={**os.environ,'TOKIO_WORKER_THREADS':'1'},stdout=subprocess.DEVNULL)
    try:
        for _ in range(100):
            if down.exists():break
            if process.poll() is not None:raise AssertionError('Proxy failed to start')
            time.sleep(.02)
        def connect():
            client=socket.socket(socket.AF_UNIX);client.settimeout(2);client.connect(str(down));server,_=listener.accept();server.settimeout(2)
            request=message(1,1,uint(2));client.sendall(request);assert recv(server)==request
            for number,name,version in [(1,'wl_compositor',4),(2,'zwlr_layer_shell_v1',4),(3,'zwlr_screencopy_manager_v1',3),(4,'wl_data_device_manager',3),(5,'unrecognized_future_protocol',1)]:
                server.sendall(message(2,0,uint(number)+string(name)+uint(version)))
            for expected in ['wl_compositor','zwlr_layer_shell_v1']:assert expected.encode() in recv(client)
            client.settimeout(.15)
            try: recv(client);raise AssertionError('Forbidden global advertised')
            except socket.timeout:pass
            client.settimeout(2)
            return client,server
        for number,name in [(3,'zwlr_screencopy_manager_v1'),(4,'wl_data_device_manager'),(5,'unrecognized_future_protocol'),(1,'zwlr_screencopy_manager_v1')]:
            client,server=connect()
            client.sendall(message(2,0,uint(number)+string(name)+uint(1)+uint(3)))
            try:recv(client);raise AssertionError('Forbidden bind survived')
            except (EOFError,ConnectionResetError):pass
            assert server.recv(1)==b'', 'Forbidden bind reached compositor'
            client.close();server.close()
        for layer in [1,3]:
            client,server=connect()
            for number,name,obj in [(1,'wl_compositor',3),(2,'zwlr_layer_shell_v1',4)]:
                request=message(2,0,uint(number)+string(name)+uint(4)+uint(obj));client.sendall(request);assert recv(server)==request
            request=message(3,0,uint(5));client.sendall(request);assert recv(server)==request
            request=message(4,0,uint(6)+uint(5)+uint(0)+uint(layer)+string('widget-test'));client.sendall(request)
            if layer==1:
                assert recv(server)==request, 'Valid bottom-layer surface blocked'
                # Exclusive keyboard capture must be rejected as well.
                client.sendall(message(6,4,uint(1)))
            error=recv(client);assert struct.unpack('=I',error[:4])[0]==1 and b'Rejected by wl-mitm' in error
            server.settimeout(.15)
            try:recv(server);raise AssertionError('Forbidden layer/keyboard request forwarded')
            except socket.timeout:pass
            client.close();server.close()
        print('PASS: actual proxy hides forbidden/unknown globals, rejects forged binds, allows bottom layer, rejects overlay and keyboard capture')
    finally:
        process.terminate();process.wait(timeout=5)
