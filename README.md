# kwoopla

`kwoopla` is a Rust command-line utility and library for discovering HomePlug AV/AV2 powerline adapters and reading network information and PHY rates.

The current implementation is read-only. It has been tested with TP-Link TL-PA9020P adapters using Broadcom BCM60500 hardware.

## Platform support

| Platform | Status |
| --- | --- |
| Linux | Supported through raw `AF_PACKET` Ethernet sockets |
| OpenWrt / kooWrt | Tested on aarch64 |
| Windows | Not yet supported; a native Npcap transport is planned |
| WSL2 | Builds successfully, but the default NAT network does not expose the physical HomePlug Layer-2 segment |

Raw Ethernet access requires root or an equivalent `CAP_NET_RAW` capability.

## Build

Rust 1.85 or newer is required.

```sh
cargo build --release
```

The resulting binary is `target/release/kwoopla`.

## Usage

Choose the Ethernet interface connected to the powerline network:

```sh
sudo ./target/release/kwoopla -i eth0 discover
sudo ./target/release/kwoopla -i eth0 info
sudo ./target/release/kwoopla -i eth0 rates
```

Use `--json` for structured, versioned output:

```sh
sudo ./target/release/kwoopla -i eth0 --json discover
sudo ./target/release/kwoopla -i eth0 --json info
sudo ./target/release/kwoopla -i eth0 --json rates
```

### `discover`

Find HomePlug devices visible on the selected Ethernet segment.

### `info`

Read station capabilities and supported vendor-specific network information.

Broadcom BCM60500 devices are queried through their Mediaxtream management path.

### `rates`

Read average transmit and receive PHY rates for visible peers.

## Current scope

Supported and live-tested paths include:

- HomePlug AV discovery and station capabilities over EtherType `0x88e1`;
- HomePlug `CM_NW_STATS` PHY-rate queries;
- Broadcom / Mediaxtream network information over EtherType `0x8912`;
- TP-Link TL-PA9020P / Broadcom BCM60500.

Network key changes, restart, factory reset, and other mutating operations are intentionally not implemented yet.

For protocol-source and interoperability notes, see [docs/provenance.md](docs/provenance.md).