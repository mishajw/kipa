#!/usr/bin/env sh

# Connects KIPA to a default hardcoded entry point.
#
# KIPA can connect to any node in the network, but this script supplies an easy way to connect to
# the author's hosted node.

set -e
DEFAULT_KEY_ID="D959094C"

if ! command -v kipa >/dev/null; then
  echo "KIPA not installed."
  exit 1
fi

key_id=${1:-$DEFAULT_KEY_ID}
key_file="$(pwd)/resources/keys/$key_id.asc"
ip_address_file="$(pwd)/resources/keys/$key_id-ip-address.txt"

if [ ! -e "$key_file" ] || [ ! -e "$ip_address_file" ]; then
  echo "Couldn't find $key_file or $ip_address_file"
  exit 1
fi

if ! gpg --list-keys $key_id >/dev/null; then
  read -p "Key ID $key_id does not exist in GPG. Import? [y/N] " result
  if [[ "$result" != "y" ]]; then
    exit
  fi
  gpg --import $key_file
fi

kipa connect \
  --key-id "$key_id" \
  --address "$(cat $ip_address_file)"
