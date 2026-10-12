# kwoopla

`kwoopla` is a Rust command-line utility and library for discovering HomePlug AV/AV2 powerline adapters and reading network information and PHY rates.

It is currently read-only and has been tested with TP-Link TL-PA9020P adapters using Broadcom BCM60500 hardware.

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

## Supported protocol paths

- HomePlug AV discovery and station capabilities over EtherType `0x88e1`.
- HomePlug `CM_NW_STATS` PHY-rate queries.
- Broadcom / Mediaxtream network information over EtherType `0x8912`.
- TP-Link TL-PA9020P / Broadcom BCM60500 live testing.

Network key changes, restart, factory reset, and other write operations are not implemented.

See [protocol provenance](docs/provenance.md) for protocol sources and interoperability evidence.

## License

Licensed under either of the following, at your option:

- [MIT](LICENSE-MIT)
- [Apache License 2.0](LICENSE-APACHE)
