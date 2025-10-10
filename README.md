# ICMPMolester Usage Guide

## Prerequisites
- Rust toolchain 1.78+ (if running natively).
- ICMP and traceroute permissions (Linux typically needs `CAP_NET_RAW`, macOS requires sudo for traceroute).
- Outbound SMTP or Telegram access when using notifications.

## Configuration
Create a `lines.toml` based on `lines.example.toml`. Each entry defines a broadband line to probe:

```toml
[defaults]
ping_count = 5
packet_loss_alert_threshold = 1.0

[[lines]]
name = "Primary FTTH"
target = "8.8.8.8"
ping_count = 8
```

### Field reference
- `ping_count`: ICMP echo attempts per run.
- `ping_timeout_ms`: per-request timeout (platform dependent).
- `traceroute_max_hops`: cap on hop depth.
- `packet_loss_alert_threshold`: percentage that triggers an alert status.

## Running Locally
```sh
cargo run -- --config lines.toml
cargo run -- --config lines.toml --skip-traceroute
```
Outputs a per-line summary with latency and loss indicators. The process exits non-zero if ping or traceroute commands fail to launch.

### Email & Telegram Notifications
Provide credentials via flags; omit them to stay CLI-only.

```sh
cargo run -- \
  --config lines.toml \
  --email-smtp smtp.example.com \
  --email-from bot@example.com \
  --email-to netops@example.com \
  --email-username bot@example.com \
  --email-password 'secret'
```

```sh
cargo run -- \
  --config lines.toml \
  --telegram-token "123456:ABC" \
  --telegram-chat-id "-1000123456"
```

Summary text is reused across transports. Telegram messages are truncated at 4096 chars.

## Docker Workflow
```sh
docker build -t icmpmolester .
docker run --rm \
  --cap-add NET_RAW \
  --cap-add NET_ADMIN \
  -v $(pwd)/lines.toml:/app/lines.toml \
  icmpmolester --config /app/lines.toml
```
Granting raw socket caps enables ping inside the container. Add env vars or secrets management for SMTP/Telegram credentials.

## Testing
```sh
cargo test -- --nocapture
```
Unit tests cover config parsing, metric extraction, and notification helpers. For integration checks, run the binary against known hosts (`ping` requires reachable addresses and ICMP access).

## Troubleshooting
- `Operation not permitted`: the OS blocked ICMP; run as root or add `CAP_NET_RAW`.
- `ping: cannot resolve host`: DNS failure—ensure connectivity or use raw IP targets.
- SMTP errors usually signal incorrect credentials or blocked outbound port 587/465.
