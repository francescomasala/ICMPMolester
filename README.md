# ICMPMolester

ICMPMolester is an asynchronous command-line tool that checks latency, packet
loss, and hop counts across multiple broadband lines. It runs `ping` and
`traceroute` concurrently for each configured circuit, summarises the results in
your terminal, and can optionally forward a concise report via email or
Telegram.

## Features
- Async execution built on Tokio for low-overhead, multi-line probing.
- Configurable per-line settings for ping count, timeouts, traceroute depth, and
  alert thresholds.
- CLI summaries that highlight packet-loss breaches and show hop counts.
- Optional email (SMTP) and Telegram notifications, reusing the same summary
  text.
- Docker-ready image for environments where raw socket access is isolated.

## Quick Start

1. **Install prerequisites**
   - Rust toolchain 1.78+.
   - ICMP/traceroute permissions (Linux usually needs `CAP_NET_RAW`; macOS
     requires sudo for traceroute).
   - Network reachability to your probe targets and, if using notifications,
     outbound SMTP or Telegram access.

2. **Create a configuration file**
   Copy `lines.example.toml` to `lines.toml` and adjust to match your circuits:

   ```toml
   [defaults]
   ping_count = 5
   ping_timeout_ms = 1000
   traceroute_max_hops = 30
   packet_loss_alert_threshold = 1.5

   [[lines]]
   name = "Primary FTTH"
   target = "8.8.8.8"
   ping_count = 8
   ```

3. **Run diagnostics**

   ```sh
   cargo run -- --config lines.toml
   cargo run -- --config lines.toml --skip-traceroute   # ping-only
   ```

   The CLI prints latency, packet loss, and hop counts. Non-zero exit codes
   indicate a ping/traceroute command failure.

4. **Send notifications (optional)**

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

   Telegram messages are truncated at 4096 characters to satisfy API limits.

## Docker Usage

```sh
docker build -t icmpmolester .
docker run --rm \
  --cap-add NET_RAW \
  --cap-add NET_ADMIN \
  -v $(pwd)/lines.toml:/app/lines.toml \
  icmpmolester --config /app/lines.toml
```

Grant raw socket capabilities so the container can invoke `ping`. Pass SMTP or
Telegram credentials via environment variables or secrets management as
required.

## Development

```sh
cargo fmt
cargo clippy -- -D warnings
cargo test -- --nocapture
```

Unit tests cover config parsing, metric extraction (including hop counts), and
notification helpers. For live network checks, run the binary against trusted
hosts in a controlled environment.

## Troubleshooting
- `Operation not permitted`: missing raw socket permissions—add `CAP_NET_RAW` or
  run with elevated privileges.
- `ping: cannot resolve host`: DNS failure; verify connectivity or use IP
  addresses.
- SMTP authentication failures commonly stem from invalid credentials or blocked
  outbound ports (587/465).

## Security

Refer to [SECURITY.md](SECURITY.md) for vulnerability reporting guidelines.

## Contributing

Community contributions are welcome. Please open an issue to propose significant
changes before raising a pull request. All code must pass `cargo fmt`, `cargo
clippy -- -D warnings`, and `cargo test -- --nocapture`.

## License

Released under the [MIT License](LICENSE).
