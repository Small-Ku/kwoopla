# kwoopla

`kwoopla` is a Rust command-line utility and library for discovering HomePlug AV/AV2 powerline adapters and reading network information and PHY rates.

The current implementation is read-only. It has been tested with TP-Link TL-PA9020P adapters using Broadcom BCM60500 hardware.

## Platform support

| Platform | Status |
| --- | --- |
| Linux | Supported through raw `AF_PACKET` Ethernet sockets |
| OpenWrt / kooWrt | Tested on aarch64 |
| Windows | Supported through Npcap |
| WSL2 | Builds successfully, but the default NAT network does not expose the physical HomePlug Layer-2 segment |

On Linux, raw Ethernet access requires root or an equivalent `CAP_NET_RAW` capability. On Windows, install the Npcap runtime before running `kwoopla`.

## Build

Rust 1.85 or newer is required. Development toolchains are managed with mise. Run the repository tasks directly; mise installs missing configured tools automatically. The Cranelift build uses the dated nightly declared in `mise.toml`, and Mr Boxington is pinned by the project configuration and lockfile.

The normal development build uses the locked nightly Cranelift backend for the workspace crate and LLVM with `opt-level = 3` for dependencies. Check, Clippy, tests, and release builds stay on stable LLVM and run through Mr Boxington:

```sh
mise run build
mise run check
mise run clippy
mise run test
mise run build-release
```

On Windows, install the Npcap runtime before running `kwoopla`. The Npcap SDK is managed by mise as a Windows-only project tool; build tasks set `LIBPCAP_LIBDIR` from its mise install path automatically.

The release binary is `target/release/kwoopla` on Linux and `target\release\kwoopla.exe` on Windows. The Npcap SDK remains outside the repository.

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

On Windows, `-i` accepts a Windows interface friendly name, adapter description, Npcap device name, or adapter GUID:

```powershell
.\target\release\kwoopla.exe -i 'Ethernet' discover
.\target\release\kwoopla.exe -i 'Ethernet' --json rates
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