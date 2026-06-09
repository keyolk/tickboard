.PHONY: help setup run build release test lint fmt fmt-check check smoke verify clean

BIN := tickboard
RELEASE_BIN := target/release/$(BIN)

help:
	@printf '%s\n' 'tickboard Make targets:'
	@printf '  %-12s %s\n' 'setup' 'Build release binary and prepare run.sh'
	@printf '  %-12s %s\n' 'run' 'Run the TUI'
	@printf '  %-12s %s\n' 'build' 'Build debug binary'
	@printf '  %-12s %s\n' 'release' 'Build release binary'
	@printf '  %-12s %s\n' 'test' 'Run unit tests'
	@printf '  %-12s %s\n' 'lint' 'Run clippy with warnings denied'
	@printf '  %-12s %s\n' 'fmt' 'Format Rust code'
	@printf '  %-12s %s\n' 'fmt-check' 'Check Rust formatting'
	@printf '  %-12s %s\n' 'check' 'Run fmt-check, lint, and tests'
	@printf '  %-12s %s\n' 'smoke' 'Launch TUI in a pseudo-terminal and quit'
	@printf '  %-12s %s\n' 'verify' 'Run check, release build, and smoke test'
	@printf '  %-12s %s\n' 'clean' 'Remove Rust build artifacts'

setup: release
	chmod +x run.sh

run:
	@if [ -x "$(RELEASE_BIN)" ]; then \
		"$(RELEASE_BIN)"; \
	else \
		cargo run --release; \
	fi

build:
	cargo build

release:
	cargo build --release

test:
	cargo test

lint:
	cargo clippy -- -D warnings

fmt:
	cargo fmt

fmt-check:
	cargo fmt -- --check

check: fmt-check lint test

smoke: release
	@if command -v script >/dev/null 2>&1; then \
		printf 'q' | script -q /dev/null "$(RELEASE_BIN)" >/dev/null; \
	else \
		echo 'script command not found; run make run manually for smoke testing'; \
		exit 1; \
	fi

verify: check release smoke

clean:
	rm -rf target
