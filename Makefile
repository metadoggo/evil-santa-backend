build:
	export RELEASE_VERSION=$(shell date +"%Y-%m-%d_%T" )
	@cargo build

release:
	export RELEASE_VERSION=$(shell git symbolic-ref -q --short HEAD || git describe --tags --exact-match || date +"%Y-%m-%d_%T" )
	@cargo build --release

clean:
	@cargo clean

TESTS = ""
test:
	@cargo test $(TESTS) --offline -- --color=always --test-threads=1 --nocapture

docs: build
	@cargo doc --no-deps

style-check:
	@rustup component add rustfmt 2> /dev/null
	cargo fmt --all -- --check

lint:
	@rustup component add clippy 2> /dev/null
	touch src/**
	cargo clippy --all-targets --all-features -- -D warnings

dev:
	@cargo run

.PHONY: build test docs style-check lint
