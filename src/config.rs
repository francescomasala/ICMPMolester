//! Configuration parsing for ICMPMolester.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

const DEFAULT_PING_COUNT: u32 = 5;
const DEFAULT_PING_TIMEOUT_MS: u64 = 1_000;
const DEFAULT_TRACEROUTE_MAX_HOPS: u8 = 30;
const DEFAULT_PACKET_LOSS_ALERT_THRESHOLD: f32 = 1.0;

/// Root configuration containing all broadband lines to probe.
#[derive(Debug)]
pub struct Config {
    pub lines: Vec<LineSettings>,
}

/// Fully-resolved per-line settings after defaults are applied.
#[derive(Debug, Clone)]
pub struct LineSettings {
    pub name: String,
    pub target: String,
    pub ping_count: u32,
    pub ping_timeout_ms: u64,
    pub traceroute_max_hops: u8,
    pub packet_loss_alert_threshold: f32,
}

#[derive(Debug, Deserialize)]
struct FileConfig {
    #[serde(default)]
    defaults: LineDefaults,
    lines: Vec<LineConfig>,
}

#[derive(Debug, Deserialize, Default)]
struct LineDefaults {
    #[serde(default)]
    ping_count: Option<u32>,
    #[serde(default)]
    ping_timeout_ms: Option<u64>,
    #[serde(default)]
    traceroute_max_hops: Option<u8>,
    #[serde(default)]
    packet_loss_alert_threshold: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct LineConfig {
    name: String,
    target: String,
    #[serde(default)]
    ping_count: Option<u32>,
    #[serde(default)]
    ping_timeout_ms: Option<u64>,
    #[serde(default)]
    traceroute_max_hops: Option<u8>,
    #[serde(default)]
    packet_loss_alert_threshold: Option<f32>,
}

impl LineDefaults {
    fn apply(&self, line: &LineConfig) -> LineSettings {
        LineSettings {
            name: line.name.clone(),
            target: line.target.clone(),
            ping_count: line
                .ping_count
                .or(self.ping_count)
                .unwrap_or(DEFAULT_PING_COUNT),
            ping_timeout_ms: line
                .ping_timeout_ms
                .or(self.ping_timeout_ms)
                .unwrap_or(DEFAULT_PING_TIMEOUT_MS),
            traceroute_max_hops: line
                .traceroute_max_hops
                .or(self.traceroute_max_hops)
                .unwrap_or(DEFAULT_TRACEROUTE_MAX_HOPS),
            packet_loss_alert_threshold: line
                .packet_loss_alert_threshold
                .or(self.packet_loss_alert_threshold)
                .unwrap_or(DEFAULT_PACKET_LOSS_ALERT_THRESHOLD),
        }
    }
}

/// Load ICMPMolester configuration from the provided TOML file.
pub fn load_config(path: &Path) -> Result<Config> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config from {}", path.display()))?;
    let parsed: FileConfig = toml::from_str(&raw)
        .with_context(|| format!("Failed to parse TOML config at {}", path.display()))?;
    if parsed.lines.is_empty() {
        anyhow::bail!("No lines defined in config {}", path.display());
    }
    let defaults = parsed.defaults;
    let lines = parsed
        .lines
        .iter()
        .map(|line| defaults.apply(line))
        .collect();
    Ok(Config { lines })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_defaults_and_overrides() {
        let contents = r#"
            [defaults]
            ping_count = 10
            ping_timeout_ms = 1500
            traceroute_max_hops = 20

            [[lines]]
            name = "Line A"
            target = "8.8.8.8"

            [[lines]]
            name = "Line B"
            target = "1.1.1.1"
            ping_count = 4
        "#;

        let parsed: FileConfig = toml::from_str(contents).unwrap();
        let defaults = parsed.defaults;
        let settings: Vec<_> = parsed
            .lines
            .iter()
            .map(|line| defaults.apply(line))
            .collect();

        assert_eq!(settings.len(), 2);
        assert_eq!(settings[0].ping_count, 10);
        assert_eq!(settings[0].ping_timeout_ms, 1500);
        assert_eq!(settings[1].ping_count, 4);
        assert_eq!(settings[1].traceroute_max_hops, 20);
    }
}
