# syntax=docker/dockerfile:experimental
# -- Base Image --
# Installs application dependencies
FROM rust:slim-buster as builder

ARG VERSION

ENV VERSION=$VERSION

# Install dependencies
RUN apt update \
 && apt install -y pkg-config libssl-dev \
 && rustup component add rustfmt

# Set up application environment
RUN cargo new --bin evm-transaction-listener
WORKDIR /evm-transaction-listener
COPY ./Cargo.toml ./Cargo.lock ./
RUN cargo build --release \
 && rm -r ./src
COPY ./src ./src
RUN rm ./target/release/deps/evm_transaction_listener* \
 && cargo build --release

# -- Test Image --
# Code to be mounted into /app
FROM builder AS test
ENTRYPOINT ["./scripts/entry.sh", "test"]

# -- Production Image --
# Runs the service
FROM debian:buster-slim AS prod
WORKDIR /app
RUN apt update \
 && apt install -y --force-yes ca-certificates
COPY ./scripts ./scripts
COPY --from=builder /evm-transaction-listener/target/release/evm-transaction-listener /app/evm-transaction-listener
ENTRYPOINT ["./scripts/entry.sh"]
