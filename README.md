# samsungtv

Samsung TV remote control CLI — legacy Tizen websocket protocol (tested
against a 2016 UN65KU6300).

## Setup

Env vars (`SAMSUNGTV_HOST` / `SAMSUNGTV_MAC` / `SAMSUNGTV_NAME`) or
`~/.config/samsungtv/config.toml`:

```toml
host = "192.168.1.42"
mac = "c0:97:27:aa:bb:cc"   # needed for `on` (wake-on-LAN)
name = "samsungtv-cli"      # client name shown on the TV (optional)
```

First connection from a new client name (or source IP): accept the
authorization popup on the TV screen.

## Usage

```bash
samsungtv status        # power state (on/standby/unreachable) + model info
samsungtv info          # full REST device info JSON
samsungtv on            # wake via WoL magic packets (unicast + broadcast)
samsungtv off           # send KEY_POWER
samsungtv key volup     # send any key (KEY_ prefix optional)
samsungtv keys          # list common key codes
samsungtv raw '{"method":"ms.remote.control","params":{...}}'  # debugging
```

`--json` switches formatted output to raw JSON.

## Notes

- The REST info API (`:8001/api/v2/`) answers even in network standby;
  "screen on" is detected by whether the remote websocket authorizes
  (`ms.channel.connect`) — same heuristic Home Assistant uses.
- Samsung TVs reject remote websocket clients from another subnet with an
  instant `ms.channel.timeOut`. If the TV sits on a different VLAN, NAT the
  client traffic so it appears to come from the TV's subnet (router
  masquerade rule for ports 8001/8002).
