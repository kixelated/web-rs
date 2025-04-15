#!/usr/bin/env just --justfile

# Using Just: https://github.com/casey/just?tab=readme-ov-file#installation

export RUST_BACKTRACE := "1"
export RUST_LOG := "debug"

# List all of the available commands.
default:
  just --list

# Install any required dependencies.
setup:
	# Make sure the WASM target is installed.
	rustup target add wasm32-unknown-unknown

	# Make sure the right components are installed.
	rustup component add rustfmt clippy

	# Install cargo shear
	cargo install cargo-shear

	# Install cargo sort
	cargo install cargo-sort

	# Install cargo-upgrades and cargo-edit
	cargo install cargo-upgrades cargo-edit

# Run the CI checks
check:
	cargo check --all-targets --all-features
	cargo clippy --all-targets --all-features -- -D warnings
	cargo fmt -- --check

	# requires: cargo install cargo-shear
	cargo shear

	# requires: cargo install cargo-sort
	cargo sort --workspace --check

# Run any CI tests
test:
	cargo test

# Automatically fix some issues.
fix:
	cargo fix --allow-staged --all-targets --all-features
	cargo clippy --fix --allow-staged --all-targets --all-features

	# requires: cargo install cargo-shear
	cargo shear --fix

	# requires: cargo install cargo-sort
	cargo sort --workspace

	cargo fmt --all

# Upgrade any tooling
upgrade:
	rustup upgrade

	# Requires: cargo install cargo-upgrades cargo-edit
	cargo upgrade