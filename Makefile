LEVEL ?= release
OUTPUT ?= bin/sshs

GO_PACKAGE_PATH := github.com/quantumsheep/sshs
VERSION := $$(git describe --tags 2>/dev/null || true)
BUILD := $$(git rev-parse --short HEAD)

ifeq ($(VERSION),)
	VERSION := latest
endif

ifeq ($(BUILD),)
	BUILD := dev
endif

ifeq ($(LEVEL),release)
	GOLDFLAGS := -w -s
endif

build:
	go build -ldflags "$(GOLDFLAGS) -X '$(GO_PACKAGE_PATH)/cmd.Version=$(VERSION)' -X '$(GO_PACKAGE_PATH)/cmd.Build=$(BUILD)'" -o $(OUTPUT)

clean:
	rm -f sshs

default: build

.PHONY: clean
