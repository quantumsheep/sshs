package ui

import (
	"crypto/sha256"
	"fmt"
	"log"
	"os"
	"os/exec"
	"regexp"
	"sort"
	"strconv"
	"strings"
	_ "unsafe"

	valid "github.com/asaskevich/govalidator"
	"github.com/gdamore/tcell/v2"
	"github.com/mikkeloscar/sshconfig"
	"github.com/rivo/tview"
)

type Host struct {
	Name         string
	User         string
	HostName     string
	ProxyCommand string
	Port         string
}

type HostsTable struct {
	*tview.Table

	Hosts            []Host
	filter           string
	displayFullProxy bool
}

//go:linkname colorPattern github.com/rivo/tview.colorPattern
var colorPattern *regexp.Regexp

func init() {
	// Shady patch to disable color pattern matching in tview
	colorPattern = regexp.MustCompile(`$^`)

	// Rounded borders
	tview.Borders.TopLeft = '╭'
	tview.Borders.TopRight = '╮'
	tview.Borders.BottomLeft = '╰'
	tview.Borders.BottomRight = '╯'

	// Set focused border style to be the same as unfocused
	tview.Borders.HorizontalFocus = tview.Borders.Horizontal
	tview.Borders.VerticalFocus = tview.Borders.Vertical
	tview.Borders.TopLeftFocus = tview.Borders.TopLeft
	tview.Borders.TopRightFocus = tview.Borders.TopRight
	tview.Borders.BottomLeftFocus = tview.Borders.BottomLeft
	tview.Borders.BottomRightFocus = tview.Borders.BottomRight
}

func connect(name string, configPath string) {
	cmd := exec.Command("ssh", "-F", configPath, strings.TrimSpace(name))
	cmd.Stdin = os.Stdin
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr

	err := cmd.Run()
	if err != nil {
		os.Exit(cmd.ProcessState.ExitCode())
	}

	os.Exit(0)
}

func asSha256(o interface{}) string {
	h := sha256.New()
	h.Write([]byte(fmt.Sprintf("%v", o)))

	return fmt.Sprintf("%x", h.Sum(nil))
}

func NewHostsTable(app *tview.Application, sshConfigPath string, filter string, sortFlag bool, displayFullProxy bool) *HostsTable {
	hosts, e := sshconfig.ParseSSHConfig(sshConfigPath)
	if e != nil {
		log.Fatal(e)
	}

	table := &HostsTable{
		Table:            tview.NewTable(),
		Hosts:            make([]Host, 0),
		filter:           strings.ToLower(filter),
		displayFullProxy: displayFullProxy,
	}

	table.
		SetBorders(false).
		SetSelectable(true, false).
		Select(0, 0).
		SetFixed(1, 1).
		SetSeparator('│').
		SetBorder(true)

	table.SetBackgroundColor(tcell.ColorReset)

	table.SetInputCapture(func(event *tcell.EventKey) *tcell.EventKey {
		switch event.Key() {
		case tcell.KeyEnter:
			row, _ := table.GetSelection()
			hostname := table.GetCell(row, 0).Text

			// In case no host is selected
			if len(hostname) > 0 {
				app.Stop()
				connect(hostname, sshConfigPath)
			}
		}

		return event
	})

	for _, host := range hosts {
		name := strings.Join(host.Host, " ")
		if name == "" {
			continue
		}

		if name[0] == '"' && name[len(name)-1] == '"' {
			name = name[1 : len(name)-1]
		}

		if host.HostName == "" && host.ProxyCommand == "" {
			if valid.IsIP(name) || valid.IsDNSName(name) {
				host.HostName = name
			} else {
				continue
			}
		}

		item := Host{
			Name:         name,
			User:         host.User,
			HostName:     host.HostName,
			ProxyCommand: host.ProxyCommand,
			Port:         strconv.Itoa(host.Port),
		}

		itemSha256 := asSha256(item)
		duplicate := false

		for _, existing := range table.Hosts {
			if asSha256(existing) == itemSha256 {
				duplicate = true
				break
			}
		}

		if !duplicate {
			table.Hosts = append(table.Hosts, item)
		}
	}

	if sortFlag {
		sort.Slice(table.Hosts, func(i, j int) bool {
			return strings.ToLower(table.Hosts[i].Name) < strings.ToLower(table.Hosts[j].Name)
		})
	}

	return table.Generate()
}

func (t *HostsTable) SetDisplayFullProxy(value bool) *HostsTable {
	t.displayFullProxy = value
	return t
}

func (t *HostsTable) Filter(filter string) *HostsTable {
	if filter != t.filter {
		t.filter = strings.ToLower(filter)
		t.Generate()
	}

	return t
}

func (t *HostsTable) Generate() *HostsTable {
	t.Clear()

	headers := []string{"Hostname", "User", "Target", "Port"}

	for col, header := range headers {
		cell := tview.NewTableCell(padding(header)).
			SetSelectable(false).
			SetTextColor(tcell.ColorBlue)

		t.SetCell(0, col, cell)
	}

	t.SetCell(0, len(headers), tview.NewTableCell("").SetSelectable(false).SetExpansion(1))

	columnsCount := t.GetColumnCount()
	selected := make([]string, columnsCount)

	row, _ := t.GetSelection()
	for col := 0; col < columnsCount; col++ {
		selected[col] = t.GetCell(row, col).Text
	}

	for _, host := range t.Hosts {
		target := host.HostName
		if target == "" {
			if host.ProxyCommand == "" {
				continue
			}

			if t.displayFullProxy {
				target = host.ProxyCommand
			} else {
				target = "(Proxy)"
			}
		}

		if !strings.Contains(strings.ToLower(host.Name), t.filter) && !strings.Contains(strings.ToLower(target), t.filter) {
			continue
		}

		values := []string{host.Name, host.User, target, host.Port}
		row := t.GetRowCount()

		isPreviouslySelected := true

		for col, value := range values {
			cell := tview.NewTableCell(padding(value)).
				SetTextColor(tcell.ColorWhite)

			t.SetCell(row, col, cell)

			if selected[col] != value {
				isPreviouslySelected = false
			}
		}

		if isPreviouslySelected {
			t.Select(row, 0)
		}

		t.SetCell(row, len(values), tview.NewTableCell("").SetExpansion(1))
	}

	return t
}

func padding(text string) string {
	return " " + text + " "
}

func (t *HostsTable) InputHandler() func(event *tcell.EventKey, setFocus func(p tview.Primitive)) {
	return t.WrapInputHandler(func(event *tcell.EventKey, setFocus func(p tview.Primitive)) {
		key := event.Key()

		switch key {
		case tcell.KeyRune:
			switch event.Rune() {
			case 'g', 'G', 'j', 'k', 'h', 'l':
				return
			}
		case tcell.KeyLeft, tcell.KeyRight:
			return
		default:
			t.Table.InputHandler()(event, setFocus)
		}
	})
}
