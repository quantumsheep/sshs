package cmd

import (
	"fmt"
	"log"
	"os"

	"github.com/mitchellh/go-homedir"
	"github.com/quantumsheep/sshs/ui"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"

	"github.com/rivo/tview"
)

var (
	Version string
)

var rootCmd = &cobra.Command{
	Use:     "sshs",
	Short:   "ssh clients manager",
	Version: Version,
	Run:     run,
}

func run(cmd *cobra.Command, args []string) {
	flags := cmd.Flags()

	sshConfigPath := "~/.ssh/config"

	if str, e := flags.GetString("config"); e == nil && str != "" {
		sshConfigPath = str
	}

	sshConfigPath, e := homedir.Expand(sshConfigPath)
	if e != nil {
		log.Fatal(e)
	}

	app := tview.NewApplication()

	displayFullProxy := false
	if proxyFlag, e := flags.GetBool("proxy"); e == nil {
		displayFullProxy = proxyFlag
	}

	filter := ""
	if str, e := flags.GetString("search"); e == nil && str != "" {
		filter = str
	}

	table := ui.NewHostsTable(app, sshConfigPath, "", displayFullProxy)

	searchBar := ui.NewSearchBar(filter)

	searchBar.SetChangedFunc(func(text string) {
		table.Filter(text)
	})

	flex := ui.NewMultiFlex().
		AddItem(searchBar, 3, 0, true).
		AddItem(table, 0, 1, true)

	flex.SetDirection(tview.FlexRow)

	if err := app.SetRoot(flex, true).SetFocus(flex).Run(); err != nil {
		panic(err)
	}
}

func Execute() {
	if err := rootCmd.Execute(); err != nil {
		fmt.Println(err)
		os.Exit(1)
	}
}

func init() {
	flags := rootCmd.PersistentFlags()
	flags.StringP("search", "s", "", "Host search filter")
	flags.StringP("config", "c", "~/.ssh/config", "SSH config file")
	flags.BoolP("proxy", "p", false, "Display full ProxyCommand")

	viper.SetDefault("author", "quantumsheep <nathanael.dmc@outlook.fr>")
	viper.SetDefault("license", "MIT")
}
