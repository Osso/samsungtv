mod config;
mod keys;
mod rest;
mod wol;
mod ws;

use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json::{Value, json};

use config::Config;

#[derive(Parser)]
#[command(name = "samsungtv", about = "Samsung TV remote control CLI", version)]
struct Cli {
    /// Output raw JSON instead of formatted text
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show model/name plus power state (on / standby / unreachable)
    Status,
    /// Print the TV's full device info from the REST API
    Info,
    /// Power on via wake-on-LAN magic packets
    On,
    /// Power off (sends KEY_POWER over the remote channel)
    Off,
    /// Send any KEY_* code, e.g. `samsungtv key volup`
    Key { key: String },
    /// List common key codes
    Keys,
    /// Send a raw JSON payload over the remote channel (debugging)
    Raw { payload: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Die quietly on closed pipes (`samsungtv keys | head`) instead of
    // panicking; Rust ignores SIGPIPE by default.
    // SAFETY: restoring the default disposition for SIGPIPE is async-signal-safe.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    let cli = Cli::parse();
    match cli.command {
        Command::Keys => {
            print_keys();
            Ok(())
        }
        command => run(command, cli.json).await,
    }
}

async fn run(command: Command, raw_json: bool) -> Result<()> {
    let config = Config::load()?;
    match command {
        Command::Status => status(&config, raw_json).await?,
        Command::Info => {
            let info = rest::device_info(&config).await?;
            println!("{}", serde_json::to_string_pretty(&info)?);
        }
        Command::On => {
            let mac = config.require_mac()?;
            let sent = wol::wake(&config.host, mac)?;
            for target in sent {
                println!("magic packet ({mac}) -> {target}");
            }
        }
        Command::Off => {
            ws::send_key(&config, "KEY_POWER").await?;
            println!("KEY_POWER sent");
        }
        Command::Key { key } => {
            let key = keys::normalize(&key);
            ws::send_key(&config, &key).await?;
            println!("{key} sent");
        }
        Command::Raw { payload } => ws::send_raw(&config, &payload).await?,
        Command::Keys => unreachable!("handled in main"),
    }
    Ok(())
}

async fn status(config: &Config, raw_json: bool) -> Result<()> {
    let info = rest::device_info(config).await.ok();
    let power = match &info {
        None => "unreachable",
        Some(_) => match ws::power_state(config).await {
            ws::Power::On => "on",
            ws::Power::Standby => "standby",
        },
    };

    if raw_json {
        let output = json!({
            "power": power,
            "info": info.unwrap_or(Value::Null),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    println!("power    {power}");
    let Some(info) = info else {
        return Ok(());
    };
    let device = &info["device"];
    for (label, value) in [
        ("name", &device["name"]),
        ("model", &device["modelName"]),
        ("ip", &device["ip"]),
        ("mac", &device["wifiMac"]),
    ] {
        if let Some(text) = value.as_str() {
            println!("{label:8} {text}");
        }
    }
    Ok(())
}

fn print_keys() {
    for (key, description) in keys::COMMON_KEYS {
        println!("{key:12}  {description}");
    }
}
