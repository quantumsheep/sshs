package ssh

import "strings"

func ParseHosts(hosts []string) string {
	s := strings.Join(hosts, " ")

	var output string

	isInString := false
	for _, c := range s {
		if c == '"' {
			if isInString {
				break
			}

			isInString = true
			continue
		}

		output += string(c)
	}

	return output
}
