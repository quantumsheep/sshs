package main

import (
	"fmt"
	"log"
	"os"
	"os/user"
	"path/filepath"
	"strings"

	"github.com/mikkeloscar/sshconfig"
	"github.com/quantumsheep/sshs/internal/display"
	"github.com/quantumsheep/sshs/internal/ssh"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

var (
	Version string
)

var rootCmd = &cobra.Command{
	Use:     "sshs",
	Short:   "SSH servers manager",
	Long:    "SSHS lets you manage your SSH servers and connect to them easily using a TUI.\nGitHub: https://github.com/quantumsheep/sshs",
	Version: Version,
	Run:     run,
}

func run(cmd *cobra.Command, args []string) {
	flags := cmd.Flags()

	sshConfigPath, err := flags.GetString("config")
	if err != nil {
		log.Fatal(err)
	}
	if sshConfigPath == "" {
		log.Fatal("empty config path")
	}
	if strings.HasPrefix(sshConfigPath, "~/") {
		currentUser, err := user.Current()
		if err != nil {
			log.Fatal(err)
		}

		sshConfigPath = filepath.Join(currentUser.HomeDir, sshConfigPath[2:])
	}

	absoluteSSHConfigPath, err := filepath.Abs(sshConfigPath)
	if err != nil {
		log.Fatal(err)
	}

	if sshConfigPath == "~/.ssh/config" {
		// Create the file if it doesn't exist
		_, err = os.Stat(sshConfigPath)
		if os.IsNotExist(err) {
			err := os.MkdirAll(filepath.Dir(absoluteSSHConfigPath), 0700)
			if err != nil {
				log.Fatal(err)
			}

			_, err = os.Create(absoluteSSHConfigPath)
			if err != nil {
				log.Fatal(err)
			}
		}
	}

	shouldDisplayProxyCommand, err := flags.GetBool("proxy")
	if err != nil {
		log.Fatal(err)
	}

	searchFilter, err := flags.GetString("search")
	if err != nil {
		log.Fatal(err)
	}

	exitAfterSessionEnds, err := flags.GetBool("exit")
	if err != nil {
		log.Fatal(err)
	}

	additionalSSHArguments, err := flags.GetString("ssh-arguments")
	if err != nil {
		log.Fatal(err)
	}

	hosts, err := sshconfig.Parse(absoluteSSHConfigPath)
	if err != nil {
		log.Fatal(err)
	}

	var d *display.Display
	d = display.NewDisplay(&display.DisplayConfig{
		SSHHosts: hosts,

		ShouldDisplayProxyCommand: shouldDisplayProxyCommand,
		SearchFilter:              searchFilter,

		OnSSHHostSelected: func(host *sshconfig.SSHHost) {
			d.Pause()

			sshHost := ssh.ParseHosts(host.Host)
			fmt.Printf("Connecting to %s...\n", sshHost)
			ssh.Run(sshHost, absoluteSSHConfigPath, additionalSSHArguments)

			if exitAfterSessionEnds {
				d.Stop()
				return
			}

			d.Resume()
		},
	})

	err = d.Start()
	if err != nil {
		log.Fatal(err)
	}
}

func init() {
	flags := rootCmd.PersistentFlags()
	flags.StringP("config", "c", "~/.ssh/config", "SSH config file")
	flags.StringP("ssh-arguments", "a", "", "Arguments for the ssh command (example: '-D 1080')")
	flags.StringP("search", "s", "", "Host search filter")
	flags.BoolP("proxy", "p", false, "Display full ProxyCommand")
	flags.BoolP("exit", "e", false, "Exit when the ssh command terminated")

	viper.SetDefault("author", "Nathanael Demacon <nathanael.dmc@outlook.fr>")
	viper.SetDefault("license", "MIT")
}

func main() {
	if e := rootCmd.Execute(); e != nil {
		fmt.Println(e)
		os.Exit(1)
	}
}
