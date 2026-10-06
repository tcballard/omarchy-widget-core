package main

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net"
	"time"
)

type geometry struct {
	Monitor             string
	X, Y, Width, Height float64
	Column, Row         int
}
type settings struct{ Username, Palette string }
type placement struct {
	Enabled       bool
	Size, Monitor string
	Workspace     *int
	Settings      settings
}
type entry struct {
	InstanceID string `json:"instanceId"`
	Placement  placement
	Effective  *geometry
}
type grid struct{ Cell, GapX, GapY float64 }
type snapshot struct {
	API       int
	Installed []entry
	Runtime   struct {
		Shown   *bool
		Editing bool
	}
	Desktop struct {
		Available bool
		Monitors  map[string]int
		Grids     map[string]grid
		Frame     struct{ Radius, BorderWidth float32 }
	}
	Palette map[string]string
}
type day struct {
	Date                  string
	Count, Level, Weekday int
}
type calendar struct {
	Username string
	Total    int
	Days     []day
}
type provider struct {
	State, Error string
	Data         *calendar
	Refreshing   bool
}

func active(s snapshot, e entry, reveal bool) bool {
	if !e.Placement.Enabled || e.Effective == nil || !s.Desktop.Available || (s.Runtime.Shown != nil && !*s.Runtime.Shown && !reveal) {
		return false
	}
	w, ok := s.Desktop.Monitors[e.Effective.Monitor]
	return ok && (e.Placement.Workspace == nil || *e.Placement.Workspace == w)
}
func request(ctx context.Context, path string, args []string, out any) error {
	data, err := json.Marshal(args)
	if err != nil {
		return err
	}
	if len(data) > 16384 {
		return fmt.Errorf("broker request too large")
	}
	d := net.Dialer{Timeout: 2 * time.Second}
	c, err := d.DialContext(ctx, "unix", path)
	if err != nil {
		return err
	}
	defer c.Close()
	stop := context.AfterFunc(ctx, func() { c.Close() })
	defer stop()
	if err = c.SetDeadline(time.Now().Add(8 * time.Second)); err != nil {
		return err
	}
	if _, err = c.Write(data); err != nil {
		return err
	}
	if err = c.(*net.UnixConn).CloseWrite(); err != nil {
		return err
	}
	data, err = io.ReadAll(io.LimitReader(c, 2*1024*1024+1))
	if err != nil {
		return err
	}
	if len(data) > 2*1024*1024 {
		return fmt.Errorf("broker response too large")
	}
	var failure struct{ Error, State string }
	if err = json.Unmarshal(data, &failure); err == nil && failure.Error != "" && failure.State == "" {
		return fmt.Errorf("%s", failure.Error)
	}
	if out == nil {
		return nil
	}
	return json.Unmarshal(data, out)
}
func placeArgs(e entry, col, row int, size, monitor string) []string {
	b, _ := json.Marshal(map[string]any{"size": size, "monitor": monitor, "column": col, "row": row, "workspace": e.Placement.Workspace})
	return []string{"place", e.InstanceID, string(b)}
}
