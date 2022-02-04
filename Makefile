VERSION ?= $$(git describe --tags 2>/dev/null || git rev-parse --short HEAD 2>/dev/null || echo "latest")
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

PREFIX ?= /usr/local

install: sshs
	mkdir -p $(DESTDIR)$(PREFIX)/bin
	cp $< $(DESTDIR)$(PREFIX)/bin/sshs

uninstall:
	rm -f $(DESTDIR)$(PREFIX)/bin/sshs

default: build

.PHONY: clean install uninstall
