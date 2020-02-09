# KIPA daemon docker image.
#
# Example usage:
#   docker build -t kipa .
#   docker run --name kipa \
#     --mount type=bind,source=$KEY_PATH,target=/root/key \
#     --mount type=bind,source=$KEY_PASSWORD_PATH,target=/root/key-password \
#     kipa $KEY_ID

FROM rust:slim-buster

RUN \
  apt-get update && \
  apt-get -y install --no-install-recommends \
    clang make automake libc-dev libclang-dev pkg-config curl gnupg protobuf-compiler \
    libgmp-dev nettle-dev

WORKDIR /root/kipa
COPY Cargo.lock Cargo.lock
COPY Cargo.toml Cargo.toml
# Fetch dependencies first, improve Docker caching performance
RUN cargo fetch

COPY build.rs build.rs
COPY resources/proto/proto_api.proto resources/proto/proto_api.proto
COPY src src
RUN cargo install --path . --root out

# ~~~~~~~~~~~~~~~~~~~~~

FROM debian:buster-slim

RUN \
  apt-get update && \
  apt-get -y install --no-install-recommends \
    gnupg && \
  apt-get -y clean && \
  rm -rf /var/lib/apt/lists/*

COPY resources/docker-run.sh resources/docker-run.sh
COPY resources/keys resources/keys

COPY --from=0 /root/kipa/out/ /

ENTRYPOINT ["sh", "resources/docker-run.sh"]
