//! Notification helpers for email and Telegram delivery.

use anyhow::{Context, Result, anyhow};
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

/// Runtime configuration required to deliver email notifications.
pub struct EmailConfig {
    pub smtp_server: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub from: String,
    pub to: Vec<String>,
}

/// Runtime configuration required to deliver Telegram notifications.
pub struct TelegramConfig {
    pub token: String,
    pub chat_id: String,
}

/// Send the textual summary via SMTP using the supplied credentials.
pub fn send_email(summary: &str, config: &EmailConfig) -> Result<()> {
    let mut builder = Message::builder()
        .from(parse_mailbox(&config.from).context("Invalid sender email address")?)
        .subject("ICMPMolester report");

    for recipient in &config.to {
        builder = builder.to(parse_mailbox(recipient).context("Invalid recipient email address")?);
    }

    let email = builder
        .body(summary.to_string())
        .context("Failed to build email message body")?;

    let mut transport_builder = SmtpTransport::relay(&config.smtp_server)
        .with_context(|| format!("Failed to configure SMTP relay {}", config.smtp_server))?;

    if let (Some(username), Some(password)) = (&config.username, &config.password) {
        transport_builder =
            transport_builder.credentials(Credentials::new(username.clone(), password.clone()));
    }

    let mailer = transport_builder.build();
    mailer
        .send(&email)
        .context("Failed to send email notification via SMTP")?;

    Ok(())
}

/// Send the textual summary via the Telegram Bot API.
pub fn send_telegram(summary: &str, config: &TelegramConfig) -> Result<()> {
    let url = format!(
        "https://api.telegram.org/bot{}/sendMessage",
        config.token.trim()
    );

    let mut body = summary.to_string();
    ensure_telegram_size(&mut body);

    let response = ureq::post(&url).send_json(ureq::json!({
        "chat_id": config.chat_id,
        "text": body,
        "disable_web_page_preview": true,
    }));

    match response {
        Ok(resp) => {
            let status = resp.status();
            if (200..300).contains(&status) {
                Ok(())
            } else {
                let text = resp
                    .into_string()
                    .unwrap_or_else(|_| "<no body>".to_string());
                Err(anyhow!(
                    "Telegram API responded with status {}: {}",
                    status,
                    text
                ))
            }
        }
        Err(ureq::Error::Status(code, resp)) => {
            let text = resp
                .into_string()
                .unwrap_or_else(|_| "<no body>".to_string());
            Err(anyhow!(
                "Telegram API responded with status {}: {}",
                code,
                text
            ))
        }
        Err(err) => Err(anyhow!(err).context("Failed to call Telegram API")),
    }
}

fn parse_mailbox(value: &str) -> Result<Mailbox> {
    value.parse::<Mailbox>().map_err(|err| anyhow!(err))
}

fn ensure_telegram_size(body: &mut String) {
    // Telegram limits messages to 4096 UTF-8 chars.
    const MAX_LEN: usize = 4096;
    if body.len() > MAX_LEN {
        body.truncate(MAX_LEN.saturating_sub(3));
        body.push_str("...");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_long_messages() {
        let mut long = "a".repeat(5000);
        ensure_telegram_size(&mut long);
        assert_eq!(long.len(), 4096);
        assert!(long.ends_with("..."));
    }

    #[test]
    fn keeps_short_messages() {
        let mut short = String::from("ok");
        ensure_telegram_size(&mut short);
        assert_eq!(short, "ok");
    }
}
