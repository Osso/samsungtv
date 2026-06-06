use std::time::Duration;

use anyhow::{Context, Result, bail};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::Message;

use crate::config::Config;

type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
/// Grace period before closing so the TV processes the key (mirrors what
/// other client libraries do; closing immediately can drop the command).
const SEND_GRACE: Duration = Duration::from_millis(700);

/// Screen power state as far as the remote channel can tell.
pub enum Power {
    /// Websocket authorized (ms.channel.connect) — screen is on.
    On,
    /// TCP/ws unreachable or channel refused — off or network standby.
    Standby,
}

/// Probe whether the screen is on by attempting a remote-channel connect.
/// This is the same heuristic Home Assistant uses for this model.
pub async fn power_state(config: &Config) -> Power {
    match connect(config).await {
        Ok(mut socket) => {
            let _ = socket.close(None).await;
            Power::On
        }
        Err(_) => Power::Standby,
    }
}

/// Send one KEY_* code over the remote channel.
pub async fn send_key(config: &Config, key: &str) -> Result<()> {
    let mut socket = connect(config).await?;
    let payload = json!({
        "method": "ms.remote.control",
        "params": {
            "Cmd": "Click",
            "DataOfCmd": key,
            "Option": "false",
            "TypeOfRemote": "SendRemoteKey",
        },
    });
    socket
        .send(Message::text(payload.to_string()))
        .await
        .with_context(|| format!("sending {key}"))?;
    // The TV reports auth/permission failures as events after the send.
    drain_events(&mut socket, SEND_GRACE).await?;
    let _ = socket.close(None).await;
    Ok(())
}

/// Send a raw JSON payload and print every event received for ~2s.
pub async fn send_raw(config: &Config, payload: &str) -> Result<()> {
    let value: Value = serde_json::from_str(payload).context("parsing payload as JSON")?;
    let mut socket = connect(config).await?;
    socket
        .send(Message::text(value.to_string()))
        .await
        .context("sending payload")?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    loop {
        let message = tokio::time::timeout_at(deadline, socket.next()).await;
        match message {
            Ok(Some(message)) => {
                if let Message::Text(text) = message.context("reading websocket message")? {
                    println!("{text}");
                }
            }
            Ok(None) | Err(_) => break, // closed or deadline reached
        }
    }
    let _ = socket.close(None).await;
    Ok(())
}

/// Open the remote channel and wait until the TV authorizes the client.
async fn connect(config: &Config) -> Result<Socket> {
    let url = config.ws_url();
    let connected = tokio::time::timeout(CONNECT_TIMEOUT, async {
        let (mut socket, _) = tokio_tungstenite::connect_async(&url)
            .await
            .with_context(|| format!("connecting to {url}"))?;
        wait_for_authorization(&mut socket).await?;
        Ok::<_, anyhow::Error>(socket)
    })
    .await;
    match connected {
        Ok(result) => result,
        Err(_) => bail!("timed out connecting to {url} — TV off or unreachable"),
    }
}

async fn wait_for_authorization(socket: &mut Socket) -> Result<()> {
    while let Some(message) = socket.next().await {
        let Message::Text(text) = message.context("reading handshake message")? else {
            continue;
        };
        let value: Value = serde_json::from_str(&text).context("parsing handshake event")?;
        check_event(&value)?;
        if value["event"].as_str() == Some("ms.channel.connect") {
            return Ok(());
        }
    }
    bail!("websocket closed before the TV authorized the connection");
}

/// Drain any events the TV pushes within `window`, surfacing errors.
async fn drain_events(socket: &mut Socket, window: Duration) -> Result<()> {
    let deadline = tokio::time::Instant::now() + window;
    while let Ok(Some(message)) = tokio::time::timeout_at(deadline, socket.next()).await {
        let Message::Text(text) = message.context("reading websocket message")? else {
            continue;
        };
        let value: Value = serde_json::from_str(&text).context("parsing event")?;
        check_event(&value)?;
    }
    Ok(())
}

/// Map known TV rejection events to actionable errors.
fn check_event(event: &Value) -> Result<()> {
    match event["event"].as_str() {
        Some("ms.channel.timeOut") => {
            bail!(
                "TV rejected the connection (ms.channel.timeOut) — Samsung TVs refuse \
                 remote clients from another subnet; see the router masquerade rule"
            )
        }
        Some("ms.error") => {
            let message = event["data"]["message"].as_str().unwrap_or("unknown error");
            if message.contains("No Authorized") {
                bail!("not authorized — accept the connection popup on the TV screen");
            }
            bail!("TV error: {message}");
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_timeout_maps_to_subnet_hint() {
        let event = json!({"event": "ms.channel.timeOut"});
        let err = check_event(&event).unwrap_err().to_string();
        assert!(err.contains("another subnet"), "got: {err}");
    }

    #[test]
    fn no_authorized_maps_to_popup_hint() {
        let event = json!({"event": "ms.error", "data": {"message": "No Authorized"}});
        let err = check_event(&event).unwrap_err().to_string();
        assert!(err.contains("popup"), "got: {err}");
    }

    #[test]
    fn connect_event_is_ok() {
        let event = json!({"event": "ms.channel.connect", "data": {}});
        assert!(check_event(&event).is_ok());
    }
}
