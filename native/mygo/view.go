package main

import (
	"fmt"
	"github.com/egoist/mygo/ui"
	"math"
	"time"
)

func (s *surface) move(col, row int, size, monitor string) {
	s.host.action(placeArgs(s.entry, col, row, size, monitor)...)
}
func (s *surface) view(c *ui.Context) {
	t := *c.Theme()
	p := s.host.snapshot.Palette
	t.Background = ui.Hex("#171a20")
	t.Text = ui.Hex("#e5e7eb")
	t.Accent = ui.Hex("#8caaee")
	if p["background"] != "" {
		t.Background = ui.Hex(p["background"])
	}
	if p["foreground"] != "" {
		t.Text = ui.Hex(p["foreground"])
	}
	if p["accent"] != "" {
		t.Accent = ui.Hex(p["accent"])
	}
	t.Surface = t.Background.Mix(t.Text, .06)
	t.TextMuted = t.Background.Mix(t.Text, .6)
	t.Border = t.Background.Mix(t.Text, .18)
	c.SetTheme(&t)
	radius := s.host.snapshot.Desktop.Frame.Radius
	frame := ui.Column(c).Fill().Padding(12).Gap(5).Radius(radius).Background(t.Background).Border(max(1, s.host.snapshot.Desktop.Frame.BorderWidth), t.Border).Focusable().Label("GitHub widget")
	if frame.Shortcut(0, ui.KeyEscape) {
		s.host.action("control", "finish-arrange")
	}
	frame.Children(func() {
		if s.host.snapshot.Runtime.Editing && s.entry.Effective != nil {
			s.arrange(c)
			return
		}
		ui.Row(c).AlignItems(ui.Center).Children(func() {
			ui.Text(c, "GitHub").FontSize(12).Bold()
			ui.Spacer(c)
			if ui.Button(c, "⚙").Label("Widget settings").Clicked() {
				s.host.action("edit", s.entry.InstanceID)
			}
		})
		d := s.data.Data
		if d == nil {
			ui.Text(c, "@"+s.entry.Placement.Settings.Username).FontSize(12)
			status := "Loading contributions…"
			if s.data.Error != "" {
				status = "Contributions unavailable"
			}
			ui.Text(c, status).FontSize(11).TextColor(t.TextMuted)
			if s.data.Error != "" {
				ui.Text(c, s.data.Error).FontSize(10).TextColor(t.TextMuted)
			}
			return
		}
		ui.Row(c).AlignItems(ui.Center).Gap(8).Children(func() {
			ui.Text(c, fmt.Sprint(d.Total)).FontSize(24).Bold()
			ui.Text(c, "contributions").FontSize(10).TextColor(t.TextMuted)
		})
		ui.Text(c, "@"+d.Username).FontSize(10).TextColor(t.TextMuted)
		for i, part := range sections(d.Days, s.entry.Placement.Size) {
			s.heatmap(c, part, i)
		}
		if s.data.State == "stale" {
			ui.Text(c, "Stale · last successful calendar").FontSize(9).TextColor(t.TextMuted)
		}
	})
}
func (s *surface) arrange(c *ui.Context) {
	g := *s.entry.Effective
	grid := s.host.snapshot.Desktop.Grids[g.Monitor]
	handle := ui.Box(c).Height(38).FillWidth().Background(c.Theme().Surface).Focusable().Label("Move widget").Tooltip("Drag to move; arrow keys move one cell")
	handle.Children(func() { ui.Text(c, "Move GitHub").FontSize(12).Bold() })
	dx, dy, pressed := handle.Dragged()
	if pressed {
		s.dragging = true
		s.dx += dx
		s.dy += dy
	} else if s.dragging {
		s.dragging = false
		col := g.Column + int(math.Round(float64(s.dx)/(grid.Cell+grid.GapX)))
		row := g.Row + int(math.Round(float64(s.dy)/(grid.Cell+grid.GapY)))
		s.dx = 0
		s.dy = 0
		s.move(col, row, s.entry.Placement.Size, g.Monitor)
	}
	for _, k := range []struct {
		key  ui.Key
		x, y int
	}{{ui.KeyLeft, -1, 0}, {ui.KeyRight, 1, 0}, {ui.KeyUp, 0, -1}, {ui.KeyDown, 0, 1}} {
		if handle.Shortcut(0, k.key) {
			s.move(g.Column+k.x, g.Row+k.y, s.entry.Placement.Size, g.Monitor)
		}
	}
	ui.Row(c).Gap(4).Children(func() {
		if ui.Button(c, "Size").Clicked() {
			next := map[string]string{"small": "medium", "medium": "large", "large": "small"}[s.entry.Placement.Size]
			s.move(g.Column, g.Row, next, g.Monitor)
		}
		if ui.Button(c, "Screen").Clicked() {
			names := s.host.monitors()
			for i, n := range names {
				if n == g.Monitor {
					s.move(0, 0, s.entry.Placement.Size, names[(i+1)%len(names)])
					break
				}
			}
		}
	})
	ui.Row(c).Gap(4).Children(func() {
		if ui.Button(c, "Hide").Clicked() {
			s.host.action("hide", s.entry.InstanceID)
		}
		if ui.Button(c, "Done").Clicked() {
			s.host.action("control", "finish-arrange")
		}
	})
	message := s.error
	if message == "" {
		message = "Drag, then release to place"
	}
	ui.Text(c, message).FontSize(10).TextColor(c.Theme().TextMuted)
}

type week [7]*day

func sections(days []day, family string) [][]week {
	var weeks []week
	for i := range days {
		d := &days[i]
		if d.Weekday < 0 || d.Weekday > 6 {
			continue
		}
		if len(weeks) == 0 || d.Weekday == 0 {
			weeks = append(weeks, week{})
		}
		weeks[len(weeks)-1][d.Weekday] = d
	}
	if family == "small" && len(weeks) > 13 {
		weeks = weeks[len(weeks)-13:]
	}
	if family == "large" && len(weeks) > 27 {
		return [][]week{weeks[:27], weeks[27:]}
	}
	return [][]week{weeks}
}
func (s *surface) heatmap(c *ui.Context, weeks []week, section int) {
	height := float32(6)
	if s.entry.Placement.Size == "large" {
		height = 12
	}
	ui.Row(c).Key(section).Gap(2).Children(func() {
		for col, w := range weeks {
			ui.Column(c).Key(col).Grow(1).Basis(0).MinWidth(0).Gap(2).Children(func() {
				for row, d := range w {
					cell := ui.Box(c).Key(row).Height(height).Radius(1)
					if d == nil {
						cell.Invisible()
						continue
					}
					color := c.Theme().Background.Mix(c.Theme().Text, .1)
					level := max(0, min(4, d.Level))
					if level > 0 {
						if s.entry.Placement.Settings.Palette == "Theme accent" {
							color = c.Theme().Background.Mix(c.Theme().Accent, []float32{0, .25, .45, .7, 1}[level])
						} else {
							color = ui.Hex([]string{"", "#0e4429", "#006d32", "#26a641", "#39d353"}[level])
						}
					}
					cell.Background(color).Tooltip(fmt.Sprintf("%s: %d contributions", d.Date, d.Count))
				}
			})
		}
	})
}
func demo(size string) (*surface, int, int) {
	w, h := 394, 192
	if size == "small" {
		w = 192
	}
	if size == "large" {
		h = 394
	}
	host := &host{actions: make(chan []string, 16)}
	host.snapshot.Desktop.Frame.Radius = 12
	host.snapshot.Desktop.Grids = map[string]grid{"DP-1": {Cell: 192, GapX: 10, GapY: 10}}
	host.snapshot.Desktop.Monitors = map[string]int{"DP-1": 1}
	s := &surface{host: host, entry: entry{InstanceID: "demo", Placement: placement{Enabled: true, Size: size, Settings: settings{Username: "synthetic-demo", Palette: "GitHub green"}}, Effective: &geometry{Monitor: "DP-1", Width: float64(w), Height: float64(h)}}, data: provider{State: "ready", Data: &calendar{Username: "synthetic-demo"}}}
	start := time.Date(2025, 10, 5, 0, 0, 0, 0, time.UTC)
	for i := 0; i < 367; i++ {
		date := start.AddDate(0, 0, i)
		level := (i*7 + i/3) % 5
		count := level * 3
		s.data.Data.Days = append(s.data.Data.Days, day{date.Format("2006-01-02"), count, level, int(date.Weekday())})
		s.data.Data.Total += count
	}
	return s, w, h
}
