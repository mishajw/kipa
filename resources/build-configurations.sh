#!/usr/bin/env bash

# This script will build and test a series of configurations for KIPA by passing
# various `--features` to `cargo`

set -e

# List of all configurations to build and test
FEATURE_CONFIGURATIONS=( \
  "use-graph use-protobuf use-tcp use-unix-socket" \
  "use-black-hole")

# Set to error on warnings
export RUSTFLAGS="-D warnings"

# Build and test with flags to pass to cargo
build_and_test() {
  FLAGS=$@
  echo "> Building and testing with flags \"$FLAGS\""
  echo cargo build $FLAGS
  cargo build "$FLAGS"
  cargo test "$FLAGS"
}

# Test building with no features
build_and_test --no-default-features

# Test all feature configurations
for c in "${FEATURE_CONFIGURATIONS[@]}"; do
  build_and_test --features=$c
done

