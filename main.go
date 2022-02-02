package main

import (
	"fmt"

	"github.com/quantumsheep/sshs/cmd"
)

var (
	Version string = "latest"
	Build   string = "dev"
)

func main() {
	cmd.Version = fmt.Sprintf("%s, build %s", Version, Build)
	cmd.Execute()
}
