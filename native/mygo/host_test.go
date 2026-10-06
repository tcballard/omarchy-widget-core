package main

import (
	"context"
	"encoding/json"
	"errors"
	"github.com/egoist/mygo/ui"
	"io"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
	"syscall"
	"testing"
	"time"
)

func TestVisibilityAndCorePlacement(t *testing.T) {
	s, _, _ := demo("medium")
	snap := s.host.snapshot
	snap.API = 2
	snap.Desktop.Available = true
	if !active(snap, s.entry, false) {
		t.Fatal("active instance hidden")
	}
	no := false
	snap.Runtime.Shown = &no
	if active(snap, s.entry, false) || !active(snap, s.entry, true) {
		t.Fatal("global visibility/reveal")
	}
	snap.Runtime.Shown = nil
	other := 2
	s.entry.Placement.Workspace = &other
	if active(snap, s.entry, false) {
		t.Fatal("workspace leaked")
	}
	s.entry.Placement.Workspace = nil
	s.entry.Effective = nil
	if active(snap, s.entry, false) {
		t.Fatal("missing placement must hide")
	}
	s, _, _ = demo("medium")
	args := placeArgs(s.entry, 2, 3, "large", "DP-2")
	var p map[string]any
	if err := json.Unmarshal([]byte(args[2]), &p); err != nil {
		t.Fatal(err)
	}
	if args[0] != "place" || p["column"] != float64(2) || p["row"] != float64(3) || p["monitor"] != "DP-2" || p["cell"] != nil {
		t.Fatalf("wrong Core placement contract: %v", args)
	}
}
func TestNativeViewRoutesActionsToCore(t *testing.T) {
	for _, size := range []string{"small", "medium", "large"} {
		s, w, h := demo(size)
		tt := ui.NewTester(s.view, w, h)
		if !tt.HasText("@synthetic-demo") {
			t.Fatal("missing account")
		}
		if err := tt.Click("Widget settings"); err != nil {
			t.Fatal(err)
		}
		args := <-s.host.actions
		if args[0] != "edit" || args[1] != "demo" {
			t.Fatal(args)
		}
		s.host.snapshot.Runtime.Editing = true
		tt.Frame()
		if err := tt.Click("Size"); err != nil {
			t.Fatal(err)
		}
		args = <-s.host.actions
		if args[0] != "place" {
			t.Fatal(args)
		}
		if err := tt.Click("Done"); err != nil {
			t.Fatal(err)
		}
		args = <-s.host.actions
		if args[0] != "control" || args[1] != "finish-arrange" {
			t.Fatal(args)
		}
	}
}
func TestLayerAttachmentFailsBeforeMutationIfRealized(t *testing.T) {
	called := false
	a := &layerAPI{realized: func(uintptr) int32 { return 1 }, init: func(uintptr) { called = true }}
	if a.attach(1, "demo", false) == nil || called {
		t.Fatal("realized window accepted")
	}
}
func TestLayerPolicyAndCoreGeometry(t *testing.T) {
	var layer, keyboard uint32
	var zone int32
	anchors := map[uint32]int32{}
	margins := map[uint32]int32{}
	var width, height int32
	a := &layerAPI{realized: func(uintptr) int32 { return 0 }, init: func(uintptr) {}, namespace: func(uintptr, string) {}, layer: func(_ uintptr, v uint32) { layer = v }, keyboard: func(_ uintptr, v uint32) { keyboard = v }, zone: func(_ uintptr, v int32) { zone = v }, anchor: func(_ uintptr, e uint32, v int32) { anchors[e] = v }, margin: func(_ uintptr, e uint32, v int32) { margins[e] = v }, monitor: func(uintptr, uintptr) {}, size: func(_ uintptr, w, h int32) { width = w; height = h }, resize: func(uintptr, int32, int32) {}}
	if err := a.attach(1, "demo", false); err != nil {
		t.Fatal(err)
	}
	if layer != 1 || keyboard != 2 || zone != -1 || anchors[0] != 1 || anchors[2] != 1 {
		t.Fatal("wrong surface policy")
	}
	a.place(1, 2, geometry{X: 212, Y: 34, Width: 394, Height: 192})
	if width != 394 || height != 192 || margins[0] != 212 || margins[2] != 34 {
		t.Fatal("placement recomputed instead of applied")
	}
	if err := a.attach(1, "demo", true); err != nil {
		t.Fatal(err)
	}
	if layer != 3 || keyboard != 0 {
		t.Fatal("wrong reveal policy")
	}
}
func listen(t *testing.T) (net.Listener, string) {
	t.Helper()
	path := filepath.Join(t.TempDir(), "broker")
	l, e := net.Listen("unix", path)
	if errors.Is(e, syscall.EPERM) {
		t.Skip("environment denies Unix socket bind; CI executes this check")
	}
	if e != nil {
		t.Fatal(e)
	}
	t.Cleanup(func() { l.Close() })
	return l, path
}
func TestBrokerFramingErrorsAndStaleData(t *testing.T) {
	for _, response := range []string{`{"state":"stale","error":"offline","data":{"username":"old","days":[]}}`, `{"error":"authority expired"}`, strings.Repeat("x", 2*1024*1024+1)} {
		l, path := listen(t)
		done := make(chan bool, 1)
		go func() {
			c, e := l.Accept()
			if e != nil {
				return
			}
			defer c.Close()
			b, _ := io.ReadAll(c)
			done <- string(b) == `["github","instance","tom"]`
			io.WriteString(c, response)
		}()
		var p provider
		err := request(context.Background(), path, []string{"github", "instance", "tom"}, &p)
		if !<-done {
			t.Fatal("request not EOF-framed")
		}
		if strings.Contains(response, `"state"`) {
			if err != nil || p.Data.Username != "old" {
				t.Fatal("stale data discarded", err)
			}
		} else if err == nil {
			t.Fatal("invalid response accepted")
		}
	}
}

type safeLog struct {
	sync.Mutex
	strings.Builder
}

func (b *safeLog) Write(p []byte) (int, error) { b.Lock(); defer b.Unlock(); return b.Builder.Write(p) }
func (b *safeLog) text() string                { b.Lock(); defer b.Unlock(); return b.Builder.String() }
func TestWaylandLayerLifecycle(t *testing.T) {
	binary := os.Getenv("MYGO_LIVE_BINARY")
	if binary == "" {
		t.Skip("requires headless Wayland compositor; CI sets MYGO_LIVE_BINARY")
	}
	l, path := listen(t)
	s, _, _ := demo("medium")
	s.entry.InstanceID = "native-live-a"
	s.entry.Effective.Monitor = "HEADLESS-1"
	snap := s.host.snapshot
	snap.API = 2
	snap.Desktop.Available = true
	snap.Desktop.Monitors = map[string]int{"HEADLESS-1": 1}
	snap.Installed = []entry{s.entry}
	var mu sync.Mutex
	go func() {
		for {
			c, err := l.Accept()
			if err != nil {
				return
			}
			func() {
				defer c.Close()
				var args []string
				b, _ := io.ReadAll(c)
				json.Unmarshal(b, &args)
				mu.Lock()
				defer mu.Unlock()
				if len(args) > 0 && args[0] == "github" {
					json.NewEncoder(c).Encode(s.data)
				} else {
					json.NewEncoder(c).Encode(snap)
				}
			}()
		}
	}()
	log := &safeLog{}
	cmd := exec.Command(binary)
	cmd.Env = append(os.Environ(), "OMARCHY_WIDGET_SANDBOX=package-offline-v2", "OMARCHY_WIDGET_BROKER="+path, "WAYLAND_DEBUG=client", "GDK_BACKEND=wayland", "MYGO_GPU=0", "NO_AT_BRIDGE=1")
	cmd.Stdout = log
	cmd.Stderr = log
	if err := cmd.Start(); err != nil {
		t.Fatal(err)
	}
	done := make(chan error, 1)
	go func() { done <- cmd.Wait() }()
	defer func() {
		cmd.Process.Kill()
		select {
		case <-done:
		case <-time.After(time.Second):
		}
		t.Log(log.text())
	}()
	await := func(label string, fn func(string) bool) {
		t.Helper()
		deadline := time.Now().Add(15 * time.Second)
		for time.Now().Before(deadline) {
			if fn(log.text()) {
				return
			}
			select {
			case e := <-done:
				t.Fatalf("native host exited: %v\n%s", e, log.text())
			default:
			}
			time.Sleep(100 * time.Millisecond)
		}
		t.Fatalf("%s not observed\n%s", label, log.text())
	}
	await("bottom layer", func(v string) bool {
		return strings.Contains(v, `"tcballard-widget-native-live-a"`) && strings.Contains(v, "set_exclusive_zone(-1)") && strings.Contains(v, "set_keyboard_interactivity(2)") && strings.Contains(v, "set_size(394, 192)")
	})
	mu.Lock()
	snap.Installed[0].Effective = &geometry{Monitor: "HEADLESS-1", X: 212, Y: 34, Width: 192, Height: 192}
	snap.Installed[0].Placement.Size = "small"
	mu.Unlock()
	await("Core geometry update", func(v string) bool {
		return strings.Contains(v, "set_margin(34, 0, 0, 212)") && strings.Contains(v, "set_size(192, 192)")
	})
	mu.Lock()
	second := snap.Installed[0]
	second.InstanceID = "native-live-b"
	snap.Installed = append(snap.Installed, second)
	mu.Unlock()
	await("duplicate instance", func(v string) bool { return strings.Contains(v, `"tcballard-widget-native-live-b"`) })
	before := strings.Count(log.text(), "zwlr_layer_surface_v1")
	mu.Lock()
	no := false
	snap.Runtime.Shown = &no
	mu.Unlock()
	await("hide", func(v string) bool {
		return strings.Count(v, "zwlr_layer_surface_v1") > before && strings.Contains(v, ".destroy()")
	})
	if strings.Contains(log.text(), ".get_toplevel(") {
		t.Fatal("native widget created ordinary xdg toplevel")
	}
}

func TestSandboxLaunchKeepsNativeHostOffline(t *testing.T) {
	source, err := os.ReadFile("../../sandbox-launch")
	if err != nil {
		t.Fatal(err)
	}
	dir := t.TempDir()
	config := filepath.Join(dir, "core")
	os.MkdirAll(filepath.Join(config, "bin"), 0700)
	capture := filepath.Join(dir, "capture")
	os.WriteFile(capture, []byte("#!/usr/bin/bash\nprintf '%s\\n' \"$@\"\n"), 0700)
	for _, name := range []string{"Host.qml", "shell.qml", "bin/omarchy-widget", "bin/omarchy-widget-mygo"} {
		if err = os.WriteFile(filepath.Join(config, name), []byte("fixture"), 0700); err != nil {
			t.Fatal(err)
		}
	}
	for _, name := range []string{"qml", "Commons", "Ui", "package"} {
		os.Mkdir(filepath.Join(config, name), 0700)
	}
	for _, name := range []string{"broker", "display"} {
		os.WriteFile(filepath.Join(config, name), nil, 0600)
	}
	script := strings.ReplaceAll(string(source), "/usr/bin/bwrap", capture)
	script = strings.ReplaceAll(script, `-S "$3"`, `-f "$3"`)
	script = strings.ReplaceAll(script, `-S "$4"`, `-f "$4"`)
	path := filepath.Join(config, "sandbox-launch")
	os.WriteFile(path, []byte(script), 0700)
	cmd := exec.Command("bash", path, "runner", filepath.Join(config, "package"), filepath.Join(config, "broker"), filepath.Join(config, "display"))
	cmd.Env = append(os.Environ(), "OMARCHY_WIDGET_RENDERER=mygo-github-experimental")
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("%v: %s", err, out)
	}
	args := string(out)
	for _, required := range []string{"--unshare-all\n", "--clearenv\n", "--cap-drop\nALL\n", "--setenv\nGDK_BACKEND\nwayland\n", "--setenv\nMYGO_GPU\n0\n", "--ro-bind\n" + filepath.Join(config, "bin/omarchy-widget-mygo") + "\n/app/bin/omarchy-widget-mygo\n"} {
		if !strings.Contains(args, required) {
			t.Fatalf("missing native isolation argument %q", required)
		}
	}
	if !strings.HasSuffix(args, "--\n/app/bin/omarchy-widget-mygo\n") || strings.Contains(args, "--share-net") || strings.Contains(args, "--dev-bind") || strings.Contains(args, "/dev/dri") {
		t.Fatal(args)
	}
}
