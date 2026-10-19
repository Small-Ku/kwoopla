# kwoopla

`kwoopla` is a Rust command-line utility and library for discovering HomePlug AV/AV2 powerline adapters, reading network information and PHY rates, and managing Broadcom power saving.

Discovery, rates, and power-saving readback have been tested with TP-Link TL-PA9020P adapters using Broadcom BCM60500 hardware. Power-saving writes are implemented but have not yet been live-tested on these adapters.

## Platform support

| Platform | Support |
| --- | --- |
| Linux | Raw `AF_PACKET` Ethernet |
| OpenWrt | Tested on aarch64 |
| Windows | Npcap |
| WSL2 | Builds, but default NAT does not expose the physical HomePlug Layer-2 segment |

Linux requires root or `CAP_NET_RAW` for raw Ethernet access. Windows requires the Npcap runtime.

## Build

Rust 1.85 or newer is required. mise installs the configured development tools when a task needs them.

```sh
mise run build
mise run ci
mise run build-release
```

`build` uses Cranelift for development builds. Checks, tests, Clippy, release builds, and runtime tests use stable LLVM. On Windows, mise also installs the Npcap SDK and configures its library path.

## Usage

Select the Ethernet interface connected to the powerline network:

```sh
sudo ./target/release/kwoopla -i eth0 discover
sudo ./target/release/kwoopla -i eth0 info
sudo ./target/release/kwoopla -i eth0 rates
```

Add `--json` for versioned JSON output:

```sh
sudo ./target/release/kwoopla -i eth0 --json info
```

On Windows, `-i` accepts an interface friendly name, adapter description, Npcap device name, or adapter GUID.

### Commands

- `discover`: find HomePlug devices on the selected Ethernet segment.
- `info`: read station capabilities and supported vendor-specific network information.
- `rates`: read transmit and receive PHY rates for visible peers.
- `power-saving get`: read enabled state and standby timeout, optionally targeting `--device MAC`.
- `power-saving set --device MAC on|off`: change one confirmed Broadcom adapter, preserving its current standby timeout. Requires acknowledged Set/Apply and matching readback.

## Supported protocol paths

- HomePlug AV discovery and station capabilities over EtherType `0x88e1`.
- HomePlug `CM_NW_STATS` PHY-rate queries.
- Broadcom / Mediaxtream network information over EtherType `0x8912`.
- TP-Link TL-PA9020P / Broadcom BCM60500 live testing.

Network key changes, restart, factory reset, and unrelated write operations are not implemented.

### Power-saving control

Select the network interface explicitly. For example:

```sh
kwoopla -i eth0 --json power-saving get
kwoopla -i eth0 power-saving get --device 00:11:22:33:44:55
kwoopla -i eth0 power-saving set --device 00:11:22:33:44:55 off
```

The power-saving setting belongs to each powerline adapter, not the PC Ethernet NIC. Disabling may help prevent powerline standby from interfering with Wake-on-LAN, but WoL still depends on the PC NIC and the source of the Magic Packet.

Writes are always explicit per-device; the CLI refuses unknown devices, missing capabilities, unknown parameter values, unacknowledged writes and failed readback. The write path follows public protocol observations from tpPLC, but is **not yet proven with a live configuration change on BCM60500**.

See [protocol provenance](docs/provenance.md) for protocol sources and interoperability evidence.

## License

Licensed under either of the following, at your option:

- [MIT](LICENSE-MIT)
- [Apache License 2.0](LICENSE-APACHE)
