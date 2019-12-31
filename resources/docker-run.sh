#/bin/sh

# Script for docker image entrypoint.

set -e

KEY_FILE="/root/key"
KEY_PASSWORD_FILE="/root/key-password"

if [ "$#" != 1 ]; then
  echo "Usage: $1 <key ID>"
  exit 1
fi
KEY_ID=$1
if ! [ -e "$KEY_FILE" ]; then
  echo "$KEY_FILE not mounted."
  exit 1;
fi
if ! [ -e "$KEY_PASSWORD_FILE" ]; then
  echo "$KEY_PASSWORD_FILE not mounted."
  exit 1;
fi

gpg --import --batch /root/key
kipa-daemon \
  --key-id "$KEY_ID" \
  --secret-path /root/key-password \
  -vvvv
