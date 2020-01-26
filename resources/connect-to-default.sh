#!/usr/bin/env sh

# Connects KIPA to a default hardcoded entry point.
#
# KIPA can connect to any node in the network, but this script supplies an easy way to connect to
# the author's hosted node.

set -e

if ! command -v kipa >/dev/null; then
  echo "KIPA not installed."
  exit 1
fi

connect_key_id=$(curl --silent https://mishajw.com/kipa-key-id.txt)
connect_address="46.101.16.228:10842"

if ! gpg --list-keys $connect_key_id >/dev/null; then
  read -p "Key ID $connect_key_id does not exist in GPG. Import? [y/N] " result
  if [[ "$result" != "y" ]]; then
    exit
  fi
  curl --silent https://mishajw.com/kipa.asc | gpg --import
fi

kipa connect \
  --key-id "$connect_key_id" \
  --address "$connect_address"
