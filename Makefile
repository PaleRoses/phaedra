.PHONY: all fmt build check test docs servedocs

all: build

test:
	cargo nextest run
	cargo nextest run -p phaedra-escape-parser # no_std by default

check:
	cargo check
	cargo check -p phaedra-escape-parser
	cargo check -p phaedra-cell
	cargo check -p phaedra-surface
	cargo check -p phaedra-ssh

build:
	cargo build $(BUILD_OPTS) -p phaedra
	cargo build $(BUILD_OPTS) -p phaedra-gui
	cargo build $(BUILD_OPTS) -p phaedra-mux-server
	cargo build $(BUILD_OPTS) -p strip-ansi-escapes

fmt:
	cargo +nightly fmt

docs:
	ci/build-docs.sh

servedocs:
	ci/build-docs.sh serve
