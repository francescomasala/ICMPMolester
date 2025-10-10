FROM rust:1.78-alpine3.19 AS builder

WORKDIR /app

# Install build prerequisites
RUN apk add --no-cache build-base openssl-dev

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM alpine:3.19

WORKDIR /app

# Runtime dependencies: ping, traceroute, and certificates for TLS if needed later
RUN apk add --no-cache iputils traceroute ca-certificates openssl && \
    update-ca-certificates

COPY --from=builder /app/target/release/icmpmolester /usr/local/bin/icmpmolester

ENTRYPOINT ["icmpmolester"]
CMD ["--help"]
