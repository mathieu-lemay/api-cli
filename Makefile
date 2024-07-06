MAKEFLAGS    += --always-make
SHELL        := /usr/bin/env bash
.SHELLFLAGS  := -e -o pipefail -c
.NOTPARALLEL :

format:
	cargo +nightly fmt

lint:
	cargo +nightly fmt -- --check
	cargo clippy --all-targets -- -D warnings

test:
	cargo test

coverage:
	cargo tarpaulin --skip-clean --include-tests --out html
