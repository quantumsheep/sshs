VERSION ?= $$(git describe --tags 2>/dev/null || git rev-parse --short HEAD)
LEVEL ?= release
OUTPUT ?= sshs

GO_PACKAGE_PATH := github.com/quantumsheep/sshs

ifeq ($(LEVEL),release)
GOLDFLAGS := -w -s
endif

build:
	go build -ldflags "$(GOLDFLAGS) -X '$(GO_PACKAGE_PATH)/cmd.Version=$(or $(strip $(VERSION)),latest)'" -o $(OUTPUT)

clean:
	rm -f sshs

default: build

.PHONY: clean
