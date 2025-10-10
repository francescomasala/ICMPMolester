//! Command-line interface for ICMPMolester.

mod config;
mod diagnostics;
mod notify;
mod runner;

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

use crate::config::load_config;
use crate::notify::{EmailConfig, TelegramConfig, send_email, send_telegram};
use crate::runner::{RunOptions, format_summary, print_cli, run_lines};

/// Command-line arguments controlling an ICMPMolester run.
#[derive(Debug, Parser)]
#[command(name = "ICMPMolester", about = "Fixed broadband diagnostics runner")]
struct Cli {
    /// Path to the ICMPMolester configuration file
    #[arg(short, long, default_value = "lines.toml")]
    config: PathBuf,

    /// Skip traceroute checks
    #[arg(long)]
    skip_traceroute: bool,

    /// SMTP server address for email notifications (e.g. smtp.example.com)
    #[arg(long)]
    email_smtp: Option<String>,

    /// SMTP username if authentication is required
    #[arg(long)]
    email_username: Option<String>,

    /// SMTP password if authentication is required
    #[arg(long)]
    email_password: Option<String>,

    /// Sender email address for notifications
    #[arg(long)]
    email_from: Option<String>,

    /// Recipient email addresses (repeat flag or comma separated)
    #[arg(long, value_delimiter = ',')]
    email_to: Vec<String>,

    /// Telegram bot token for notifications
    #[arg(long)]
    telegram_token: Option<String>,

    /// Telegram chat ID to deliver notifications to
    #[arg(long)]
    telegram_chat_id: Option<String>,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = load_config(&cli.config)?;
    let options = RunOptions {
        skip_traceroute: cli.skip_traceroute,
    };

    let results = run_lines(config, options).await?;
    print_cli(&results);

    let summary = format_summary(&results);

    if let Some(email_cfg) = build_email_config(&cli)? {
        send_email(&summary, &email_cfg)?;
        println!(
            "Email notification dispatched to {}",
            email_cfg.to.join(", ")
        );
    }

    if let Some(telegram_cfg) = build_telegram_config(&cli)? {
        send_telegram(&summary, &telegram_cfg)?;
        println!(
            "Telegram notification dispatched to {}",
            telegram_cfg.chat_id
        );
    }

    Ok(())
}

/// Validate and construct email notification configuration when requested.
fn build_email_config(cli: &Cli) -> Result<Option<EmailConfig>> {
    let email_requested = cli.email_smtp.is_some()
        || cli.email_username.is_some()
        || cli.email_password.is_some()
        || cli.email_from.is_some()
        || !cli.email_to.is_empty();

    if !email_requested {
        return Ok(None);
    }

    let smtp = cli
        .email_smtp
        .as_ref()
        .context("Email SMTP server required when enabling email notifications")?
        .clone();
    let from = cli
        .email_from
        .as_ref()
        .context("Sender address required when enabling email notifications")?
        .clone();
    if cli.email_to.is_empty() {
        anyhow::bail!(
            "At least one --email-to recipient required when enabling email notifications"
        );
    }

    Ok(Some(EmailConfig {
        smtp_server: smtp,
        username: cli.email_username.clone(),
        password: cli.email_password.clone(),
        from,
        to: cli.email_to.clone(),
    }))
}

/// Validate and construct Telegram notification configuration when requested.
fn build_telegram_config(cli: &Cli) -> Result<Option<TelegramConfig>> {
    let telegram_requested = cli.telegram_token.is_some() || cli.telegram_chat_id.is_some();
    if !telegram_requested {
        return Ok(None);
    }

    let token = cli
        .telegram_token
        .as_ref()
        .context("Telegram bot token required when enabling Telegram notifications")?
        .clone();
    let chat_id = cli
        .telegram_chat_id
        .as_ref()
        .context("Telegram chat ID required when enabling Telegram notifications")?
        .clone();

    Ok(Some(TelegramConfig { token, chat_id }))
}
