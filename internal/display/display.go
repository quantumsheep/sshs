package display

import (
	"fmt"
	"os"
	"strings"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/mikkeloscar/sshconfig"
	"github.com/quantumsheep/sshs/internal/display/components"
	"github.com/quantumsheep/sshs/internal/ssh"
	"github.com/samber/lo"
)

type Display struct {
	Program *tea.Program
}

type DisplayConfig struct {
	SSHHosts []*sshconfig.SSHHost

	ShouldDisplayProxyCommand bool
	SearchFilter              string

	OnSSHHostSelected func(*sshconfig.SSHHost)
}

func NewDisplay(config *DisplayConfig) *Display {
	rows := lo.FilterMap(config.SSHHosts, func(host *sshconfig.SSHHost, _ int) (*components.ListItem, bool) {
		name := ssh.ParseHosts(host.Host)
		if name == "*" {
			return nil, false
		}

		var details []string

		if host.HostName != "" && name != host.HostName {
			target := host.HostName
			if host.Port != 22 {
				target += fmt.Sprintf(":%d", host.Port)
			}

			details = append(details, fmt.Sprintf("Target: %s", target))
		} else if host.Port != 22 {
			details = append(details, fmt.Sprintf("Port: %d", host.Port))
		}

		details = append(details, fmt.Sprintf("User: %s", host.User))

		if config.ShouldDisplayProxyCommand && host.ProxyCommand != "" {
			details = append(details, fmt.Sprintf("ProxyCommand: %s", host.ProxyCommand))
		}

		return &components.ListItem{
			ID: host.Host,

			Name:    name,
			Details: strings.Join(details, "\n"),
		}, true
	})

	c := components.NewListComponent(&components.ListComponentConfig{
		Items:               rows,
		DefaultSearchFilter: config.SearchFilter,
		OnSelect: func(item *components.ListItem) {
			for _, host := range config.SSHHosts {
				id := item.ID.([]string)

				if len(host.Host) != len(id) {
					continue
				}

				match := true
				for i, hostPart := range host.Host {
					if hostPart != id[i] {
						match = false
						break
					}
				}

				if match {
					config.OnSSHHostSelected(host)
					return
				}
			}
		},
	})

	program := tea.NewProgram(c)

	return &Display{
		Program: program,
	}
}

func (d *Display) Start() error {
	if _, err := d.Program.Run(); err != nil {
		fmt.Println("Error running program:", err)
		os.Exit(1)
	}

	return nil
}

func (d *Display) Pause() error {
	return d.Program.ReleaseTerminal()
}

func (d *Display) Resume() error {
	return d.Program.RestoreTerminal()
}

func (d *Display) Stop() {
	d.Program.Quit()
}
