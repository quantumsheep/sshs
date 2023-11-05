package ssh

import (
	"os"
	"os/exec"

	"github.com/google/shlex"
)

func Run(host string, configPath string, additionalArguments string) error {
	args := []string{"-F", configPath, host}
	if additionalArguments != "" {
		parsedAdditionalArguments, err := shlex.Split(additionalArguments)
		if err != nil {
			return err
		}

		args = append(args, parsedAdditionalArguments...)
	}

	command := exec.Command("ssh", args...)
	command.Stdin = os.Stdin
	command.Stdout = os.Stdout
	command.Stderr = os.Stderr

	err := command.Run()
	if err != nil {
		os.Exit(command.ProcessState.ExitCode())
	}

	return nil
}
