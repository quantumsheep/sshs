package cmd

import (
	"fmt"
	"log"
	"os"
	"path/filepath"

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
	Run:     runRoot,
}

func init() {
	flags := rootCmd.Flags()
	flags.StringP("search", "s", "", "Host search filter")
	flags.StringP("config", "c", "~/.ssh/config", "SSH config file")
	flags.BoolP("proxy", "p", false, "Display full ProxyCommand")

	viper.SetDefault("author", "quantumsheep <nathanael.dmc@outlook.fr>")
	viper.SetDefault("license", "MIT")

	rootCmd.AddCommand(generateCmd)
}

func Execute() {
	if err := rootCmd.Execute(); err != nil {
		fmt.Println(err)
		os.Exit(1)
	}
}

func runRoot(cmd *cobra.Command, args []string) {
	flags := cmd.Flags()

	sshConfigPath, e := flags.GetString("config")
	if e != nil {
		log.Fatal(e)
	}

	if sshConfigPath == "" {
		log.Fatal("empty config path")
	}

	absoluteSshConfigPath, e := homedir.Expand(sshConfigPath)
	if e != nil {
		log.Fatal(e)
	}

	if sshConfigPath == "~/.ssh/config" {
		e := createFileRecursive(absoluteSshConfigPath)
		if e != nil {
			log.Fatal(e)
		}
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

	table := ui.NewHostsTable(app, absoluteSshConfigPath, filter, displayFullProxy)

	searchBar := ui.NewSearchBar(filter)

	searchBar.SetChangedFunc(func(text string) {
		table.Filter(text)
	})

	flex := ui.NewMultiFlex().
		AddItem(searchBar, 3, 0, true).
		AddItem(table, 0, 1, true)

	flex.SetDirection(tview.FlexRow)

	if e := app.SetRoot(flex, true).SetFocus(flex).Run(); e != nil {
		panic(e)
	}
}

func createFileRecursive(filename string) error {
	if _, e := os.Stat(filename); os.IsNotExist(e) {
		if e := os.MkdirAll(filepath.Dir(filename), os.ModePerm); e != nil {
			return e
		}

		f, e := os.OpenFile(filename, os.O_RDONLY|os.O_CREATE, 0o644)
		if e != nil {
			return e
		}
		f.Close()
	}

	return nil
}
