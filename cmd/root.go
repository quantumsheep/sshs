package cmd

import (
	"fmt"
	"log"
	"os"
	"os/exec"
	"strconv"
	"strings"

	"github.com/mikkeloscar/sshconfig"
	"github.com/mitchellh/go-homedir"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"

	ui "github.com/gizak/termui/v3"
	"github.com/gizak/termui/v3/widgets"
	tb "github.com/nsf/termbox-go"
)

func connect(name string) {
	cmd := exec.Command("ssh", strings.TrimSpace(name))
	cmd.Stdin = os.Stdin
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr

	err := cmd.Run()
	if err != nil {
		os.Exit(cmd.ProcessState.ExitCode())
	}

	os.Exit(0)
}

var RootCmd = &cobra.Command{
	Use:   "sshs",
	Short: "ssh clients manager",
	Run: func(cmd *cobra.Command, args []string) {
		// read ~/.ssh/config
		filepath, e := homedir.Expand("~/.ssh/config")
		if e != nil {
			log.Fatal(e)
		}

		hosts, e := sshconfig.ParseSSHConfig(filepath)
		if e != nil {
			log.Fatal(e)
		}

		if err := ui.Init(); err != nil {
			log.Fatalf("failed to initialize termui: %v", err)
		}
		defer ui.Close()
		tb.SetInputMode(tb.InputEsc)

		searchBar := widgets.NewParagraph()
		searchBar.Text = "Search..."
		searchBar.PaddingLeft = 1
		searchBar.PaddingRight = 1
		// searchBar.BorderStyle.Fg = ui.ColorM
		searchBar.TextStyle.Fg = ui.ColorYellow

		table := widgets.NewTable()
		table.TextAlignment = ui.AlignLeft
		table.RowSeparator = false
		table.FillRow = true
		table.RowStyles[0] = ui.NewStyle(ui.ColorBlue, ui.ColorClear, ui.ModifierBold)

		table.Rows = make([][]string, 1)
		table.Rows[0] = []string{"Hostname", "User", "Host", "Port"}

		for _, host := range hosts {
			if host.HostName == "" {
				continue
			}

			name := strings.Join(host.Host, " ")

			if name[0] == '"' && name[len(name)-1] == '"' {
				name = name[1 : len(name)-1]
			}

			row := []string{name, host.User, host.HostName, strconv.Itoa(host.Port)}
			table.Rows = append(table.Rows, row)
		}

		for _, row := range table.Rows {
			for i := range row {
				row[i] = " " + row[i]
			}
		}

		table.ColumnWidths = make([]int, len(table.Rows[0]))
		padding := 1

		for _, row := range table.Rows {
			for i, cell := range row {
				length := len(cell) + padding

				if length > table.ColumnWidths[i] {
					table.ColumnWidths[i] = length
				}
			}
		}

		grid := ui.NewGrid()

		termWidth, termHeight := ui.TerminalDimensions()
		grid.SetRect(0, 0, termWidth, termHeight)

		searchBarRatio := 3 / float64(termHeight)

		grid.Set(
			ui.NewCol(1.0,
				ui.NewRow(searchBarRatio, searchBar),
				ui.NewRow(1.0-searchBarRatio, table),
			),
		)

		selectedHost := 0
		table.RowStyles[selectedHost+1] = ui.NewStyle(ui.ColorBlack, ui.ColorWhite, ui.ModifierClear)

		ui.Render(grid)

		sum := 0
		for i := 0; i < len(table.ColumnWidths)-1; i++ {
			sum += table.ColumnWidths[i]
		}

		table.ColumnWidths[len(table.ColumnWidths)-1] = table.Inner.Dx() - sum

		ui.Render(grid)

		rows := table.Rows[1:]
		previousSearch := ""
		search := ""

		previousKey := ""
		uiEvents := ui.PollEvents()
		for {
			newSelectedHost := selectedHost

			e := <-uiEvents
			switch e.ID {
			case "<C-c>":
				return
			case "<Enter>":
				ui.Close()
				connect(table.Rows[selectedHost+1][0])
				return
			case "<Resize>":
				termWidth, termHeight := ui.TerminalDimensions()
				grid.SetRect(0, 0, termWidth, termHeight)

				searchBarRatio := 3 / float64(termHeight)

				grid.Set(
					ui.NewCol(1.0,
						ui.NewRow(searchBarRatio, searchBar),
						ui.NewRow(1.0-searchBarRatio, table),
					),
				)

				ui.Clear()
			case "<Down>":
				newSelectedHost += 1
			case "<Up>":
				newSelectedHost -= 1
			case "<Backspace>":
				if len(search) > 0 {
					search = search[:len(search)-1]
				}
			case "<Space>":
				search += " "
			case "<C-u>":
				search = ""
			default:
				if len(e.ID) == 1 {
					search += e.ID
				}
			}

			if previousSearch != search {
				table.Rows = append([][]string(nil), table.Rows[:1]...)

				if len(search) > 0 {
					searchBar.Text = search
					searchBar.TextStyle.Fg = ui.ColorClear

					searchLowerCase := strings.ToLower(search)

					for _, row := range rows {
						if strings.Contains(strings.ToLower(row[0]), searchLowerCase) || strings.Contains(strings.ToLower(row[2]), searchLowerCase) {
							table.Rows = append(table.Rows, row)
						}
					}
				} else {
					searchBar.Text = "Search..."
					searchBar.TextStyle.Fg = ui.ColorYellow

					table.Rows = append(table.Rows, rows...)
				}
			}

			previousSearch = search

			maxHosts := len(table.Rows) - 1

			if maxHosts > 0 {
				newSelectedHost %= maxHosts
			} else {
				newSelectedHost = 0
			}

			if newSelectedHost < 0 {
				newSelectedHost = newSelectedHost + maxHosts
			} else if newSelectedHost >= len(rows) {
				newSelectedHost = newSelectedHost - maxHosts
			}

			if previousKey == "g" {
				previousKey = ""
			} else {
				previousKey = e.ID
			}

			if newSelectedHost != selectedHost {
				table.RowStyles[selectedHost+1] = ui.NewStyle(ui.ColorWhite, ui.ColorClear, ui.ModifierClear)
				table.RowStyles[newSelectedHost+1] = ui.NewStyle(ui.ColorBlack, ui.ColorWhite, ui.ModifierClear)
				selectedHost = newSelectedHost
			}

			ui.Render(grid)
		}
	},
}

func Execute() {
	if err := RootCmd.Execute(); err != nil {
		fmt.Println(err)
		os.Exit(1)
	}
}

func init() {
	flags := RootCmd.PersistentFlags()
	flags.StringP("search", "s", "", "Host search filter")

	viper.SetDefault("author", "quantumsheep <nathanael.dmc@outlook.fr>")
	viper.SetDefault("license", "MIT")
}
