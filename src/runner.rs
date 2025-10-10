//! Orchestrates diagnostics execution and presentation.

use anyhow::{Context, Result};

use crate::config::Config;
use crate::diagnostics::{PingReport, TracerouteReport, run_ping, run_traceroute};

/// Options that control how ICMPMolester runs diagnostics.
pub struct RunOptions {
    pub skip_traceroute: bool,
}

/// Aggregated diagnostic outcome for a single broadband line.
#[derive(Debug)]
pub struct LineResult {
    pub name: String,
    pub target: String,
    pub loss_threshold: f32,
    pub ping: PingReport,
    pub traceroute: Option<TracerouteReport>,
    pub traceroute_requested: bool,
}

/// Execute diagnostics for every configured line and collect results.
pub fn run_lines(config: Config, options: RunOptions) -> Result<Vec<LineResult>> {
    let mut results = Vec::new();

    for line in config.lines {
        let ping_report = run_ping(&line)
            .with_context(|| format!("Ping check failed for line '{}'", line.name))?;

        let traceroute_report = if options.skip_traceroute {
            None
        } else {
            Some(
                run_traceroute(&line)
                    .with_context(|| format!("Traceroute failed for line '{}'", line.name))?,
            )
        };

        results.push(LineResult {
            name: line.name,
            target: line.target,
            loss_threshold: line.packet_loss_alert_threshold,
            ping: ping_report,
            traceroute: traceroute_report,
            traceroute_requested: !options.skip_traceroute,
        });
    }

    Ok(results)
}

/// Stream a human-friendly summary of the diagnostic results to STDOUT.
pub fn print_cli(results: &[LineResult]) {
    for result in results {
        println!("=== ICMPMolester: {} ({}) ===", result.name, result.target);
        print_ping_summary(result);

        match (&result.traceroute, result.traceroute_requested) {
            (Some(report), _) => print_traceroute_summary(report),
            (None, true) => println!("Traceroute status: ALERT command failed"),
            (None, false) => println!("Traceroute: skipped"),
        }

        println!();
    }
}

/// Produce a concise text summary suitable for notifications.
pub fn format_summary(results: &[LineResult]) -> String {
    let mut summary = String::from("ICMPMolester summary\n");

    for result in results {
        let loss_text = result
            .ping
            .packet_loss_pct
            .map(|loss| format!("{loss:.2}%"))
            .unwrap_or_else(|| "n/a".into());
        let latency_text = result
            .ping
            .average_latency_ms
            .map(|latency| format!("{latency:.2} ms"))
            .unwrap_or_else(|| "n/a".into());
        let loss_status = match result.ping.packet_loss_pct {
            Some(loss) if loss > result.loss_threshold => "ALERT",
            Some(_) => "OK",
            None => "UNKNOWN",
        };
        let ping_status = if result.ping.success { "OK" } else { "ALERT" };
        let traceroute_status = match (&result.traceroute, result.traceroute_requested) {
            (Some(report), _) if report.success => "OK",
            (Some(_), _) => "ALERT",
            (None, true) => "ALERT",
            (None, false) => "SKIPPED",
        };

        summary.push_str(&format!(
            "- {} ({}): ping={ping_status}, loss={loss_text} ({loss_status}), latency={}, traceroute={}\n",
            result.name, result.target, latency_text, traceroute_status
        ));
    }

    summary
}

fn print_ping_summary(result: &LineResult) {
    println!("Ping status: {}", bool_to_status(result.ping.success));
    match result.ping.packet_loss_pct {
        Some(loss) => {
            let status = if loss > result.loss_threshold {
                "ALERT above threshold"
            } else {
                "OK within threshold"
            };
            println!("Packet loss: {loss:.2}% ({status})");
        }
        None => println!("Packet loss: unavailable"),
    }

    match result.ping.average_latency_ms {
        Some(latency) => println!("Average latency: {latency:.2} ms"),
        None => println!("Average latency: unavailable"),
    }
}

fn print_traceroute_summary(report: &TracerouteReport) {
    println!("Traceroute status: {}", bool_to_status(report.success));
    match report.raw_output.lines().next() {
        Some(line) if !line.trim().is_empty() => println!("First hop: {line}"),
        _ => println!("Traceroute output empty"),
    }
}

fn bool_to_status(success: bool) -> &'static str {
    if success {
        "OK success"
    } else {
        "ALERT command failed"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_result(
        name: &str,
        success: bool,
        packet_loss: Option<f32>,
        avg_latency: Option<f32>,
        loss_threshold: f32,
        traceroute_success: Option<bool>,
    ) -> LineResult {
        LineResult {
            name: name.into(),
            target: "10.0.0.1".into(),
            loss_threshold,
            ping: PingReport {
                success,
                packet_loss_pct: packet_loss,
                average_latency_ms: avg_latency,
                raw_output: String::new(),
            },
            traceroute: traceroute_success.map(|ok| TracerouteReport {
                success: ok,
                raw_output: String::new(),
            }),
            traceroute_requested: traceroute_success.is_some(),
        }
    }

    #[test]
    fn formats_summary_with_alerts() {
        let results = vec![
            sample_result("Primary", true, Some(0.5), Some(12.3), 1.0, Some(true)),
            sample_result("Backup", false, Some(5.0), None, 1.0, Some(false)),
            sample_result("Lab", true, None, None, 1.0, None),
        ];

        let summary = format_summary(&results);
        assert!(summary.contains("Primary"));
        assert!(summary.contains("loss=0.50% (OK)"));
        assert!(summary.contains("ping=ALERT"));
        assert!(summary.contains("traceroute=ALERT"));
        assert!(summary.contains("Lab (10.0.0.1):"));
        assert!(summary.contains("traceroute=SKIPPED"));
    }
}
