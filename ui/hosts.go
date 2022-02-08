package ui

import (
	"log"
	"os"
	"os/exec"
	"strconv"
	"strings"

	"github.com/gdamore/tcell/v2"
	"github.com/quantumsheep/sshconfig"
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

func NewHostsTable(app *tview.Application, sshConfigPath string, filter string, displayFullProxy bool) *HostsTable {
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
		SetBorders(true).
		SetSelectable(true, false).
		Select(0, 0).
		SetFixed(1, 1)

	table.SetBackgroundColor(tcell.ColorReset)

	table.SetInputCapture(func(event *tcell.EventKey) *tcell.EventKey {
		switch event.Key() {
		case tcell.KeyEnter:
			row, _ := table.GetSelection()
			hostname := table.GetCell(row, 0).Text

			app.Stop()
			connect(hostname, sshConfigPath)
		}

		return event
	})

	for _, host := range hosts {
		if host.HostName == "" && host.ProxyCommand == "" {
			continue
		}

		name := strings.Join(host.Host, " ")

		if name[0] == '"' && name[len(name)-1] == '"' {
			name = name[1 : len(name)-1]
		}

		item := Host{
			Name:         name,
			User:         host.User,
			HostName:     host.HostName,
			ProxyCommand: host.ProxyCommand,
			Port:         strconv.Itoa(host.Port),
		}

		table.Hosts = append(table.Hosts, item)
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

		for col, value := range values {
			cell := tview.NewTableCell(padding(value)).
				SetTextColor(tcell.ColorWhite)

			t.SetCell(row, col, cell)
		}

		t.SetCell(row, len(values), tview.NewTableCell("").SetExpansion(1))
	}

	return t
}

func padding(text string) string {
	return " " + text + " "
}
