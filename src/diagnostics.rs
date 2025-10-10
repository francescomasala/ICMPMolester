//! Shell-based diagnostics helpers (ping/traceroute execution and parsing).

use std::ffi::OsString;
use std::process::Command;
use std::sync::OnceLock;

use anyhow::{Context, Result};
use regex::Regex;

use crate::config::LineSettings;

/// Parsed summary of a ping operation.
#[derive(Debug)]
pub struct PingReport {
    pub success: bool,
    pub packet_loss_pct: Option<f32>,
    pub average_latency_ms: Option<f32>,
    pub raw_output: String,
}

/// Parsed summary of a traceroute operation.
#[derive(Debug)]
pub struct TracerouteReport {
    pub success: bool,
    pub raw_output: String,
}

/// Execute ping for a configured line and parse loss/latency.
pub fn run_ping(line: &LineSettings) -> Result<PingReport> {
    let mut command = Command::new(ping_command());
    for arg in ping_args(line) {
        command.arg(arg);
    }

    let output = command
        .output()
        .with_context(|| format!("Failed to execute ping for {}", line.name))?;

    let raw_output = collect_output(&output.stdout, &output.stderr);
    let packet_loss_pct = extract_packet_loss(&raw_output);
    let average_latency_ms = extract_average_latency(&raw_output);

    Ok(PingReport {
        success: output.status.success(),
        packet_loss_pct,
        average_latency_ms,
        raw_output,
    })
}

/// Execute traceroute for a configured line and capture raw output.
pub fn run_traceroute(line: &LineSettings) -> Result<TracerouteReport> {
    let mut command = Command::new(traceroute_command());
    for arg in traceroute_args(line) {
        command.arg(arg);
    }

    let output = command
        .output()
        .with_context(|| format!("Failed to execute traceroute for {}", line.name))?;
    let raw_output = collect_output(&output.stdout, &output.stderr);

    Ok(TracerouteReport {
        success: output.status.success(),
        raw_output,
    })
}

#[cfg(windows)]
fn ping_command() -> &'static str {
    "ping"
}

#[cfg(not(windows))]
fn ping_command() -> &'static str {
    "ping"
}

#[cfg(windows)]
fn ping_args(line: &LineSettings) -> Vec<OsString> {
    vec![
        OsString::from("-n"),
        OsString::from(line.ping_count.to_string()),
        OsString::from("-w"),
        OsString::from(line.ping_timeout_ms.to_string()),
        OsString::from(&line.target),
    ]
}

#[cfg(not(windows))]
fn ping_args(line: &LineSettings) -> Vec<OsString> {
    let mut args = vec![
        OsString::from("-c"),
        OsString::from(line.ping_count.to_string()),
        OsString::from(&line.target),
    ];

    if line.ping_timeout_ms > 0 {
        args.insert(2, OsString::from("-W"));
        let timeout_value = if cfg!(target_os = "linux") {
            let secs = std::cmp::max(1, (line.ping_timeout_ms + 999) / 1000);
            secs.to_string()
        } else {
            line.ping_timeout_ms.to_string()
        };
        args.insert(3, OsString::from(timeout_value));
    }

    args
}

#[cfg(windows)]
fn traceroute_command() -> &'static str {
    "tracert"
}

#[cfg(not(windows))]
fn traceroute_command() -> &'static str {
    "traceroute"
}

#[cfg(windows)]
fn traceroute_args(line: &LineSettings) -> Vec<OsString> {
    vec![
        OsString::from("-h"),
        OsString::from(line.traceroute_max_hops.to_string()),
        OsString::from(&line.target),
    ]
}

#[cfg(not(windows))]
fn traceroute_args(line: &LineSettings) -> Vec<OsString> {
    vec![
        OsString::from("-m"),
        OsString::from(line.traceroute_max_hops.to_string()),
        OsString::from(&line.target),
    ]
}

fn extract_packet_loss(output: &str) -> Option<f32> {
    static LOSS_REGEX: OnceLock<Regex> = OnceLock::new();
    let regex = LOSS_REGEX
        .get_or_init(|| Regex::new(r"(?P<loss>\d+(?:\.\d+)?)%\s*(?:packet\s+loss|loss)").unwrap());
    regex
        .captures_iter(output)
        .last()
        .and_then(|caps| caps.name("loss"))
        .and_then(|m| m.as_str().parse::<f32>().ok())
}

fn extract_average_latency(output: &str) -> Option<f32> {
    static UNIX_REGEX: OnceLock<Regex> = OnceLock::new();
    static WINDOWS_REGEX: OnceLock<Regex> = OnceLock::new();

    let unix_regex = UNIX_REGEX.get_or_init(|| Regex::new(r"= [\d\.]+/([\d\.]+)/[\d\.]+").unwrap());
    unix_regex
        .captures_iter(output)
        .last()
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().parse::<f32>().ok())
        .or_else(|| {
            let windows_regex =
                WINDOWS_REGEX.get_or_init(|| Regex::new(r"Average = (\d+)ms").unwrap());
            windows_regex
                .captures_iter(output)
                .last()
                .and_then(|caps| caps.get(1))
                .and_then(|m| m.as_str().parse::<f32>().ok())
        })
}

fn collect_output(stdout: &[u8], stderr: &[u8]) -> String {
    let mut body = String::from_utf8_lossy(stdout).to_string();
    if !stderr.is_empty() {
        if !body.is_empty() && !body.ends_with('\n') {
            body.push('\n');
        }
        body.push_str(&String::from_utf8_lossy(stderr));
    }
    body
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_unix_packet_loss_and_latency() {
        let sample = r#"
PING 8.8.8.8 (8.8.8.8) 56(84) bytes of data.
64 bytes from 8.8.8.8: icmp_seq=1 ttl=115 time=19.2 ms

--- 8.8.8.8 ping statistics ---
4 packets transmitted, 4 received, 0% packet loss, time 3004ms
rtt min/avg/max/mdev = 18.677/19.002/19.543/0.352 ms
"#;

        assert_eq!(extract_packet_loss(sample), Some(0.0));
        assert_eq!(extract_average_latency(sample), Some(19.002));
    }

    #[test]
    fn parses_windows_packet_loss_and_latency() {
        let sample = r#"
Ping statistics for 1.1.1.1:
    Packets: Sent = 4, Received = 4, Lost = 0 (0% loss),
Approximate round trip times in milli-seconds:
    Minimum = 35ms, Maximum = 40ms, Average = 37ms
"#;

        assert_eq!(extract_packet_loss(sample), Some(0.0));
        assert_eq!(extract_average_latency(sample), Some(37.0));
    }

    #[test]
    fn collects_combined_output() {
        let out = collect_output(b"hello", b"world");
        assert!(out.contains("hello"));
        assert!(out.contains("world"));
        assert!(out.contains('\n'));

        let out = collect_output(b"", b"err");
        assert_eq!(out, "err");
    }
}
