use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::config::Config;

const RETRIES: u32 = 3;

/// Fetch the TV's device info from the REST API.
///
/// This endpoint answers even in network standby, so a successful response
/// does NOT mean the screen is on (see ws::power_state for that).
pub async fn device_info(config: &Config) -> Result<Value> {
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .context("building HTTP client")?;
    let url = config.rest_url();
    let mut last_err = None;
    for attempt in 1..=RETRIES {
        match fetch(&http, &url).await {
            Ok(value) => return Ok(value),
            Err(err) if attempt < RETRIES => {
                eprintln!("warning: attempt {attempt} failed ({err}), retrying...");
                tokio::time::sleep(Duration::from_millis(500 * u64::from(attempt))).await;
                last_err = Some(err);
            }
            Err(err) => return Err(err),
        }
    }
    Err(last_err.expect("loop ran at least once"))
}

async fn fetch(http: &reqwest::Client, url: &str) -> Result<Value> {
    let resp = http
        .get(url)
        .send()
        .await
        .with_context(|| format!("requesting {url}"))?;
    let status = resp.status();
    let text = resp.text().await.context("reading response body")?;
    if !status.is_success() {
        bail!("HTTP {status}: {text}");
    }
    serde_json::from_str(&text).with_context(|| format!("parsing device info: {text}"))
}
