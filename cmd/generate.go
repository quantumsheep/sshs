package cmd

import (
	"fmt"
	"log"
	"os"
	"regexp"
	"strings"

	"github.com/mitchellh/go-homedir"
	"github.com/quantumsheep/sshconfig"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

var generateCmd = &cobra.Command{
	Use:     "generate",
	Short:   "Generate a ssh configuration from different sources",
	Version: rootCmd.Version,
	Run:     runGenerate,
}

func init() {
	flags := generateCmd.Flags()
	flags.Bool("known-hosts", false, "Generate from known_hosts file")
	flags.String("known-hosts-file", "~/.ssh/known_hosts", "Path of known_hosts file")

	viper.SetDefault("author", "quantumsheep <nathanael.dmc@outlook.fr>")
	viper.SetDefault("license", "MIT")
}

func runGenerate(cmd *cobra.Command, args []string) {
	flags := cmd.Flags()

	knownHosts := false

	if enabled, e := flags.GetBool("known-hosts"); e == nil {
		knownHosts = enabled
	}

	if !knownHosts {
		cmd.Help()
		os.Exit(0)
	}

	configs := make([]*KnownHostConfig, 0)

	if knownHosts {
		knownHostsFile := "~/.ssh/known_hosts"

		if str, e := flags.GetString("known-hosts-file"); e == nil && str != "" {
			knownHostsFile = str
		}

		knownHostsFile, e := homedir.Expand(knownHostsFile)
		if e != nil {
			log.Fatal(e)
		}

		// open file
		bytes, e := os.ReadFile(knownHostsFile)
		if e != nil {
			log.Fatal(e)
		}

		data := string(bytes)
		rx := regexp.MustCompile(`^((\[(?P<HostWithPort>.*?)\]:(?P<Port>\d+))|((?P<DomainName>.*?),(?P<IP>.*?))|(?P<Host>.*?))[ ]`)

		lines := strings.Split(data, "\n")
		for _, line := range lines {
			if line == "" {
				continue
			}

			config := NewKnownHostConfig()

			matches := rx.FindStringSubmatch(line)

			if host := matches[rx.SubexpIndex("HostWithPort")]; host != "" {
				config.Host = host
				config.HostName = host
				config.Port = matches[rx.SubexpIndex("Port")]
			} else if host := matches[rx.SubexpIndex("DomainName")]; host != "" {
				config.Host = host
				config.HostName = matches[rx.SubexpIndex("IP")]
			} else if host := matches[rx.SubexpIndex("Host")]; host != "" {
				config.Host = host
				config.HostName = host
			}

			configs = append(configs, config)
		}
	}

	config := strings.Join(KnownHostConfigStrings(KnownHostConfigUniques(configs)), "\n\n")
	fmt.Println(config)
}

type KnownHostConfig struct {
	*sshconfig.SSHHost

	Host string
	Port string
}

func NewKnownHostConfig() *KnownHostConfig {
	return &KnownHostConfig{
		SSHHost: &sshconfig.SSHHost{},
		Port:    "22",
	}
}

func (c *KnownHostConfig) String() string {
	return "Host " + c.Host +
		"\n\tHostname " + c.HostName +
		"\n\tPort " + c.Port
}

func KnownHostConfigStrings(configs []*KnownHostConfig) []string {
	list := make([]string, 0)

	for _, item := range configs {
		list = append(list, item.String())
	}

	return list
}

func KnownHostConfigUniques(configs []*KnownHostConfig) []*KnownHostConfig {
	list := make([]*KnownHostConfig, 0)

	for _, item := range configs {
		found := false

		for _, item2 := range list {
			if item.Host == item2.Host && item.HostName == item2.HostName && item.Port == item2.Port {
				found = true
				break
			}
		}

		if !found {
			list = append(list, item)
		}
	}

	return list
}
