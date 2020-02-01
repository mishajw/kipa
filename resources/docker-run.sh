#/bin/sh

# Script for docker image entrypoint.

set -e

KEY_FILE="/root/key"
KEY_PASSWORD_FILE="/root/key-password"

if ! [ -e "$KEY_FILE" ]; then
  echo "$KEY_FILE not mounted."
  exit 1;
fi
if ! [ -e "$KEY_PASSWORD_FILE" ]; then
  echo "$KEY_PASSWORD_FILE not mounted."
  exit 1;
fi

# Import the secret key + all keys in `./resources/keys`.
gpg --import --batch /root/key
gpg --import --batch ./resources/keys/*.asc
kipa-daemon \
  -vvvv \
  --secret-path /root/key-password \
  "$@"
