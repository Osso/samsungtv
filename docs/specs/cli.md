# samsungtv CLI

Command-line remote control for legacy Tizen Samsung TVs (websocket protocol
on port 8001, no token; verified against a 2016 UN65KU6300). Source lives in
`src/`. How-it-works notes are in the README for now.

## What it must do

Config:
- [x] Load host/mac/name from `SAMSUNGTV_HOST` / `SAMSUNGTV_MAC` / `SAMSUNGTV_NAME` env vars, falling back to `~/.config/samsungtv/config.toml` `key = "value"` lines.
- [ ] `host` is required; `mac` only for `on`; `name` defaults to `samsungtv-cli`.

Commands:
- [ ] `status` — REST device info + power state (`on` when the remote websocket authorizes, `standby` when REST answers but the websocket doesn't, `unreachable` when REST fails).
- [ ] `info` — full REST device info as pretty JSON.
- [x] `on` — wake-on-LAN magic packet (6×0xFF + 16×MAC, 102 bytes) sent via UDP/9 to the host unicast, 255.255.255.255, and the host's /24 broadcast. MAC accepted in `aa:bb:..` and `AA-BB-..` forms.
- [ ] `off` — connect the remote channel, wait for `ms.channel.connect`, send `KEY_POWER`.
- [x] `key <KEY>` — send any key code; input is uppercased and `KEY_` prefixed when missing.
- [ ] `keys` — print the curated common-key list.
- [ ] `raw <json>` — send a raw payload, print events received for ~2s.
- [ ] `--json` — raw JSON output where a formatted view is the default.

Error mapping:
- [x] `ms.channel.timeOut` → error explaining Samsung's same-subnet restriction and the router masquerade.
- [x] `ms.error` / "No Authorized" → error telling the user to accept the popup on the TV.

## How it works

- README.md (protocol notes; no wiki yet).

## Implementation inventory

- `src/main.rs` — clap commands, output formatting.
- `src/config.rs` — env/file config, hand-rolled toml-line parser, base64 for the ws client name.
- `src/rest.rs` — device info fetch with retries.
- `src/ws.rs` — remote-channel connect/authorize, key send, raw send, power probe, event→error mapping.
- `src/wol.rs` — MAC parsing, magic-packet build, UDP send.
- `src/keys.rs` — key normalization + curated key list.

## Tests asserting this spec

- `src/config.rs` (config parsing, base64 vectors)
- `src/wol.rs` (magic packet, MAC parsing, subnet broadcast)
- `src/keys.rs` (key normalization)
- `src/ws.rs` (event→error mapping)

## Known gaps (current cycle)

- [ ] Live verification against the TV (status/on/off/key) — network behavior is untested by unit tests by design.

## Out of scope

- Token-auth models (2018+, `TokenAuthSupport`) — different handshake on port 8002; this TV doesn't use it.
- App launching / SmartThings API.
