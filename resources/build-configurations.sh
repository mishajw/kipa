#!/usr/bin/env bash

# This script will build and test a series of configurations for KIPA by passing
# various `--features` to `cargo`

set -e

# List of all configurations to build and test
FEATURE_CONFIGURATIONS=( \
  "use-graph use-protobuf use-tcp use-unix-socket" \
  "use-black-hole")

# Build and test with flags to pass to cargo
build_and_test() {
  echo "> Building with flags \"$@\""
  # Check for no occurances of "error" or "warning"
  if cargo check "$@" 2>&1 | grep -P "^(error|warning)" 1>/dev/null ; then
    cargo check "$@"
    exit
  fi

  echo "> Testing with flags \"$@\""
  cargo test "$@"
}

# Test building with no features
build_and_test --no-default-features

# Test all feature configurations
for c in "${FEATURE_CONFIGURATIONS[@]}"; do
  build_and_test --no-default-features --features "$c"
done

