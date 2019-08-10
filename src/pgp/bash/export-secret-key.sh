#!/usr/bin/env bash

# Given a key ID through command line and a passphrase though stdin, writes an
# unencrypted private key to stdout.
#
# Allows KIPA to use GNUPG keys in sequoia_openpgp.
#
# TODO: Convert this file to rust, or find a less painful way to export
# unencrypted private

set -e

if [[ "$#" != 1 ]]; then
  echo "Usage: $0 <key ID>"
  exit 1
fi

KEY_ID=$1
read KEY_PASSPHRASE

GPG_DIRECTORY=$(mktemp --directory)
KEY_DIRECTORY=$(mktemp --directory)
KEY_FILE="$KEY_DIRECTORY/key"
trap "rm -r $GPG_DIRECTORY $KEY_DIRECTORY" exit

# TODO: Is it safe to echo passphrase?
echo "$KEY_PASSPHRASE" | gpg \
  --pinentry-mode loopback --passphrase-fd 0 --command-fd 0 \
  --output "$KEY_FILE" --export-secret-keys "$KEY_ID"
echo "$KEY_PASSPHRASE" | gpg \
  --homedir "$GPG_DIRECTORY" \
  --pinentry-mode loopback --passphrase-fd 0 --command-fd 0 \
  --import $KEY_FILE
# TODO: why :(
(echo "$KEY_PASSPHRASE" && echo "" && echo "") | gpg \
  --homedir "$GPG_DIRECTORY" \
  --batch --pinentry-mode loopback --status-fd 0 --command-fd 0 \
  --change-passphrase "$KEY_ID"
gpg \
  --homedir "$GPG_DIRECTORY" \
  --export-secret-keys "$KEY_ID"
