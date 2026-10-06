package main

import (
	"fmt"
	"github.com/ebitengine/purego"
	"unsafe"
)

// GTK3 only, tied to MyGo v0.2.11. Every call is on MyGo's GTK main thread.
// Refuse realized windows instead of silently falling back to an xdg toplevel.
type layerAPI struct {
	supported  func() int32
	realized   func(uintptr) int32
	init       func(uintptr)
	namespace  func(uintptr, string)
	layer      func(uintptr, uint32)
	anchor     func(uintptr, uint32, int32)
	margin     func(uintptr, uint32, int32)
	zone       func(uintptr, int32)
	keyboard   func(uintptr, uint32)
	monitor    func(uintptr, uintptr)
	size       func(uintptr, int32, int32)
	resize     func(uintptr, int32, int32)
	display    func() uintptr
	screen     func(uintptr) uintptr
	count      func(uintptr) int32
	getMonitor func(uintptr, int32) uintptr
	plug       func(uintptr, int32) *byte
	free       func(*byte)
}

func loadLayer() (a *layerAPI, err error) {
	defer func() {
		if p := recover(); p != nil {
			a = nil
			err = fmt.Errorf("GTK layer-shell symbols: %v", p)
		}
	}()
	a = &layerAPI{}
	gtk, e := purego.Dlopen("libgtk-3.so.0", purego.RTLD_NOW|purego.RTLD_GLOBAL)
	if e != nil {
		return nil, e
	}
	lib, e := purego.Dlopen("libgtk-layer-shell.so.0", purego.RTLD_NOW|purego.RTLD_GLOBAL)
	if e != nil {
		return nil, fmt.Errorf("install gtk-layer-shell: %w", e)
	}
	for _, b := range []struct {
		target any
		name   string
	}{
		{&a.supported, "gtk_layer_is_supported"}, {&a.init, "gtk_layer_init_for_window"}, {&a.namespace, "gtk_layer_set_namespace"},
		{&a.layer, "gtk_layer_set_layer"}, {&a.anchor, "gtk_layer_set_anchor"}, {&a.margin, "gtk_layer_set_margin"},
		{&a.zone, "gtk_layer_set_exclusive_zone"}, {&a.keyboard, "gtk_layer_set_keyboard_mode"}, {&a.monitor, "gtk_layer_set_monitor"},
	} {
		purego.RegisterLibFunc(b.target, lib, b.name)
	}
	for _, b := range []struct {
		target any
		name   string
	}{
		{&a.realized, "gtk_widget_get_realized"}, {&a.size, "gtk_widget_set_size_request"}, {&a.resize, "gtk_window_resize"},
		{&a.display, "gdk_display_get_default"}, {&a.screen, "gdk_display_get_default_screen"}, {&a.count, "gdk_display_get_n_monitors"},
		{&a.getMonitor, "gdk_display_get_monitor"}, {&a.plug, "gdk_screen_get_monitor_plug_name"}, {&a.free, "g_free"},
	} {
		purego.RegisterLibFunc(b.target, gtk, b.name)
	}
	if a.supported() == 0 {
		return nil, fmt.Errorf("Wayland layer-shell unavailable; refusing ordinary application window")
	}
	return a, nil
}
func cString(p *byte) string {
	if p == nil {
		return ""
	}
	b := make([]byte, 0, 128)
	for i := 0; i < 1024; i++ {
		v := *(*byte)(unsafe.Add(unsafe.Pointer(p), i))
		if v == 0 {
			break
		}
		b = append(b, v)
	}
	return string(b)
}
func (a *layerAPI) findMonitor(name string) uintptr {
	d := a.display()
	s := a.screen(d)
	var found uintptr
	for i := int32(0); i < a.count(d); i++ {
		p := a.plug(s, i)
		n := cString(p)
		if p != nil {
			a.free(p)
		}
		if n == name {
			if found != 0 {
				return 0
			}
			found = a.getMonitor(d, i)
		}
	}
	return found // no index/model-name/primary-monitor fallback
}
func (a *layerAPI) attach(w uintptr, id string, reveal bool) error {
	if a.realized(w) != 0 {
		return fmt.Errorf("MyGo realized window before layer-shell attachment; pinned toolkit contract changed")
	}
	a.init(w)
	a.namespace(w, "tcballard-widget-"+id)
	layer, keyboard := uint32(1), uint32(2) // bottom, on demand
	if reveal {
		layer = 3
		keyboard = 0
	}
	a.layer(w, layer)
	a.keyboard(w, keyboard)
	a.zone(w, -1)
	a.anchor(w, 0, 1)
	a.anchor(w, 2, 1) // GTK layer shell LEFT=0 TOP=2
	return nil
}
func (a *layerAPI) place(w, monitor uintptr, g geometry) {
	a.monitor(w, monitor)
	a.margin(w, 0, int32(g.X))
	a.margin(w, 2, int32(g.Y))
	a.size(w, int32(g.Width), int32(g.Height))
	a.resize(w, 1, 1)
}
