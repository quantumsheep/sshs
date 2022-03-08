package cmd

import (
	"fmt"
	"log"
	"os"
	"regexp"
	"strings"

	valid "github.com/asaskevich/govalidator"
	"github.com/mitchellh/go-homedir"
	"github.com/quantumsheep/sshconfig"
	"github.com/spf13/cobra"
	"github.com/spf13/pflag"
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
	flags.Bool("known-hosts-allow-single-ip", true, "Allow single IP addresses (without hostname)")

	viper.SetDefault("author", "quantumsheep <nathanael.dmc@outlook.fr>")
	viper.SetDefault("license", "MIT")
}

func runGenerate(cmd *cobra.Command, args []string) {
	flags := cmd.Flags()

	knownHosts, e := flags.GetBool("known-hosts")
	if e != nil {
		log.Fatal(e)
	}

	if !knownHosts {
		cmd.Help()
		os.Exit(0)
	}

	configs := make([]*KnownHostConfig, 0)

	if knownHosts {
		configs = append(configs, generateFromKnownHosts(flags)...)
	}

	config := strings.Join(KnownHostConfigStrings(KnownHostConfigUniques(configs)), "\n\n")
	fmt.Println(config)
}

func generateFromKnownHosts(flags *pflag.FlagSet) []*KnownHostConfig {
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
	lines := strings.Split(data, "\n")

	rx := regexp.MustCompile(`^(\[(?P<Host>.*?)\]:(?P<Port>\d+))|(?P<SingleHost>.*?)$`)

	configs := make([]*KnownHostConfig, 0)

	for _, line := range lines {
		if line == "" {
			continue
		}

		lineConfigs := make([]*KnownHostConfig, 0)

		targets := strings.Split(strings.Split(line, " ")[0], ",")
		for _, target := range targets {
			config := NewKnownHostConfig()

			matches := rx.FindStringSubmatch(target)

			if host := matches[rx.SubexpIndex("Host")]; host != "" {
				port := matches[rx.SubexpIndex("Port")]

				config.Host = host + ":" + port
				config.HostName = host
				config.Port = port
			} else if host := matches[rx.SubexpIndex("SingleHost")]; host != "" {
				config.Host = host
				config.HostName = host
			}

			lineConfigs = append(lineConfigs, config)
		}

		allowSingleIp, e := flags.GetBool("known-hosts-allow-single-ip")
		if e != nil {
			log.Fatal(e)
		}

		var config *KnownHostConfig = nil

		// Select the first config with a valid domain name (defaults to the first config)
		for _, lineConfig := range lineConfigs {
			if valid.IsDNSName(lineConfig.HostName) {
				config = lineConfig
				break
			}
		}

		if config != nil {
			configs = append(configs, config)
		} else if allowSingleIp && len(lineConfigs) > 0 {
			configs = append(configs, lineConfigs[0])
		}
	}

	return configs
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
		"\n  Hostname " + c.HostName +
		"\n  Port " + c.Port
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
