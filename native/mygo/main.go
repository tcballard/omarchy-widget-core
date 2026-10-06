package main

import (
	"context"
	"flag"
	"fmt"
	"github.com/egoist/mygo"
	"github.com/egoist/mygo/ui"
	"image/png"
	"log"
	"os"
	"sort"
	"sync"
	"time"
)

type result struct {
	snapshot    snapshot
	data        map[string]provider
	err         error
	actionError string
}
type surface struct {
	host     *host
	window   *mygo.Window
	entry    entry
	data     provider
	error    string
	shown    bool
	monitor  uintptr
	geometry geometry
	dragging bool
	dx, dy   float32
}
type host struct {
	api      *layerAPI
	windows  map[string]*surface
	snapshot snapshot
	reveal   bool
	actions  chan []string
}

func (h *host) action(args ...string) {
	select {
	case h.actions <- args:
	default:
		log.Print("action queue full; retry")
	}
}
func (h *host) apply(r result) {
	if r.err != nil {
		for _, s := range h.windows {
			s.window.Hide()
			s.shown = false
		}
		log.Print(r.err)
		return
	}
	h.snapshot = r.snapshot
	wanted := map[string]bool{}
	for _, e := range r.snapshot.Installed {
		wanted[e.InstanceID] = true
		s := h.windows[e.InstanceID]
		if s == nil && !e.Placement.Enabled {
			continue
		}
		if s == nil {
			s = &surface{host: h, entry: e}
			h.windows[e.InstanceID] = s
			s.window = mygo.NewWindow(mygo.WindowOptions{Title: "GitHub Contributions", Width: 192, Height: 192, Hidden: true, Frameless: true, Transparent: true, Content: ui.View(s.view)})
			if err := h.api.attach(s.window.NativeHandle(), e.InstanceID, h.reveal); err != nil {
				log.Fatal(err)
			}
			s.window.OnClose(func(ev *mygo.CloseEvent) { ev.PreventDefault(); h.action("hide", s.entry.InstanceID) })
		}
		s.entry = e
		s.error = r.actionError
		s.data = r.data[e.InstanceID]
		if active(r.snapshot, e, h.reveal) {
			g := *e.Effective
			m := h.api.findMonitor(g.Monitor)
			if m != 0 {
				if m != s.monitor || g != s.geometry {
					h.api.place(s.window.NativeHandle(), m, g)
					s.monitor = m
					s.geometry = g
				}
				if !s.shown {
					s.window.ShowInactive()
					s.shown = true
				}
			} else {
				s.window.Hide()
				s.shown = false
				log.Printf("waiting for connector %q", g.Monitor)
			}
		} else {
			s.window.Hide()
			s.shown = false
			s.dragging = false
			s.dx = 0
			s.dy = 0
		}
		s.window.Update(func() {})
	}
	for id, s := range h.windows {
		if !wanted[id] {
			s.window.Destroy()
			delete(h.windows, id)
		}
	}
}
func worker(ctx context.Context, path string, h *host, wg *sync.WaitGroup) {
	defer wg.Done()
	ticker := time.NewTicker(time.Second)
	defer ticker.Stop()
	actionError := ""
	for {
		var snap snapshot
		err := request(ctx, path, []string{"list"}, &snap)
		if err == nil && snap.API != 2 {
			err = fmt.Errorf("unsupported Core snapshot API")
		}
		r := result{snapshot: snap, data: map[string]provider{}, err: err, actionError: actionError}
		if err == nil {
			for _, e := range snap.Installed {
				if active(snap, e, h.reveal) {
					var p provider
					if er := request(ctx, path, []string{"github", e.InstanceID, e.Placement.Settings.Username}, &p); er != nil {
						p = provider{State: "error", Error: er.Error()}
					}
					r.data[e.InstanceID] = p
				}
			}
		}
		if ctx.Err() != nil {
			return
		}
		mygo.RunOnMain(func() { h.apply(r) })
		select {
		case <-ctx.Done():
			return
		case args := <-h.actions:
			actionError = ""
			if e := request(ctx, path, args, nil); e != nil {
				actionError = e.Error()
				log.Print(e)
			}
		case <-ticker.C:
		}
	}
}
func main() {
	render := flag.String("render", "", "render synthetic Core-sized preview PNG")
	size := flag.String("size", "medium", "preview family")
	flag.Parse()
	if *render != "" {
		s, w, h := demo(*size)
		f, e := os.Create(*render)
		if e != nil {
			log.Fatal(e)
		}
		e = png.Encode(f, ui.Render(s.view, w, h, 1))
		ce := f.Close()
		if e != nil {
			log.Fatal(e)
		}
		if ce != nil {
			log.Fatal(ce)
		}
		return
	}
	path := os.Getenv("OMARCHY_WIDGET_BROKER")
	if path == "" || os.Getenv("OMARCHY_WIDGET_SANDBOX") != "package-offline-v2" {
		log.Fatal("launch through Widget Core; broker and package sandbox required")
	}
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	h := &host{windows: map[string]*surface{}, actions: make(chan []string, 16), reveal: os.Getenv("OMARCHY_WIDGET_REVEAL") == "1"}
	var wg sync.WaitGroup
	mygo.App.SetName("Widget Core · MyGo")
	mygo.App.OnWindowAllClosed(func() {}) // package may temporarily have no visible instances
	mygo.App.WhenReady(func() {
		var err error
		h.api, err = loadLayer()
		if err != nil {
			log.Fatal(err)
		}
		wg.Add(1)
		go worker(ctx, path, h, &wg)
	})
	err := mygo.App.Run()
	cancel()
	wg.Wait()
	if err != nil {
		log.Fatal(err)
	}
}
func (h *host) monitors() []string {
	a := make([]string, 0, len(h.snapshot.Desktop.Monitors))
	for k := range h.snapshot.Desktop.Monitors {
		a = append(a, k)
	}
	sort.Strings(a)
	return a
}
