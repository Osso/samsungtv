use anyhow::{Result, bail};

pub struct Config {
    pub host: String,
    pub mac: Option<String>,
    pub name: String,
}

const DEFAULT_NAME: &str = "samsungtv-cli";

impl Config {
    /// Load from SAMSUNGTV_HOST / SAMSUNGTV_MAC / SAMSUNGTV_NAME env vars,
    /// falling back to ~/.config/samsungtv/config.toml (host/mac/name keys).
    pub fn load() -> Result<Self> {
        let file = read_config_file();
        let get = |env: &str, idx: usize| {
            std::env::var(env)
                .ok()
                .or_else(|| file.as_ref().and_then(|f| f[idx].clone()))
        };
        let host = get("SAMSUNGTV_HOST", 0);
        let mac = get("SAMSUNGTV_MAC", 1);
        let name = get("SAMSUNGTV_NAME", 2).unwrap_or_else(|| DEFAULT_NAME.to_string());

        let Some(host) = host else {
            bail!(
                "missing TV host: set SAMSUNGTV_HOST, or create \
                 ~/.config/samsungtv/config.toml with a host key"
            );
        };
        Ok(Self { host, mac, name })
    }

    /// MAC address, required for wake-on-LAN.
    pub fn require_mac(&self) -> Result<&str> {
        match self.mac.as_deref() {
            Some(mac) => Ok(mac),
            None => bail!(
                "missing TV MAC address (needed for wake-on-LAN): set SAMSUNGTV_MAC, \
                 or add a mac key to ~/.config/samsungtv/config.toml"
            ),
        }
    }

    pub fn rest_url(&self) -> String {
        format!("http://{}:8001/api/v2/", self.host)
    }

    pub fn ws_url(&self) -> String {
        format!(
            "ws://{}:8001/api/v2/channels/samsung.remote.control?name={}",
            self.host,
            base64_encode(self.name.as_bytes())
        )
    }
}

fn read_config_file() -> Option<[Option<String>; 3]> {
    let home = std::env::var("HOME").ok()?;
    let path = format!("{home}/.config/samsungtv/config.toml");
    let text = std::fs::read_to_string(path).ok()?;
    Some(parse_config(&text))
}

/// Minimal key = "value" parser; avoids a toml dependency for three keys.
fn parse_config(text: &str) -> [Option<String>; 3] {
    let mut values: [Option<String>; 3] = [None, None, None];
    for line in text.lines() {
        let line = line.trim();
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().trim_matches('"').to_string();
        match key.trim() {
            "host" => values[0] = Some(value),
            "mac" => values[1] = Some(value),
            "name" => values[2] = Some(value),
            _ => {}
        }
    }
    values
}

/// Standard base64 (RFC 4648, with padding); avoids a dependency for one use.
pub fn base64_encode(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        let indices = [(n >> 18) & 63, (n >> 12) & 63, (n >> 6) & 63, n & 63];
        for (i, &index) in indices.iter().enumerate() {
            if i <= chunk.len() {
                out.push(ALPHABET[index as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_host_mac_and_name() {
        let text = "host = \"192.168.1.42\"\nmac = \"c0:97:27:aa:bb:cc\"\nname = \"mytv\"\n";
        let [host, mac, name] = parse_config(text);
        assert_eq!(host.as_deref(), Some("192.168.1.42"));
        assert_eq!(mac.as_deref(), Some("c0:97:27:aa:bb:cc"));
        assert_eq!(name.as_deref(), Some("mytv"));
    }

    #[test]
    fn ignores_unrelated_keys_and_sections() {
        let [host, mac, name] = parse_config("[server]\nfoo = 1\nbar = \"x\"\n");
        assert_eq!(host, None);
        assert_eq!(mac, None);
        assert_eq!(name, None);
    }

    #[test]
    fn base64_matches_known_vectors() {
        // RFC 4648 test vectors.
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
        // The name Home Assistant uses, seen verbatim in TV client lists.
        assert_eq!(base64_encode(b"HomeAssistant"), "SG9tZUFzc2lzdGFudA==");
    }
}
