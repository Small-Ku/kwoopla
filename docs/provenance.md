# Protocol provenance

This document records protocol sources that materially informed the current implementation. It is not a catalogue of every project inspected during research.

No third-party implementation source has been copied into this repository.

| Value or behavior | Evidence | Current use and limit |
| --- | --- | --- |
| HomePlug AV EtherType `0x88e1`, Ethernet management framing, standard message types | Public [HomePlug AV 1.1 specification](https://docbox.etsi.org/Reference/homeplug_av11/homeplug_av11_specification_final_public.pdf); cross-checked with [jbit/powerline message definitions](https://github.com/jbit/powerline/tree/main/src/homeplug) | Standard frame parsing, `CC_DISCOVER_LIST` (`0x0014` request / `0x0015` confirmation), `CM_STA_CAP` (`0x6034` / `0x6035`), and `CM_NW_STATS` (`0x6048` / `0x6049`). |
| HomePlug MMTYPE byte order and five-byte AV 1.1/2.0 header | Public specification and [jbit/powerline `message.rs`](https://github.com/jbit/powerline/blob/main/src/homeplug/message.rs) | MMTYPE is encoded little-endian after MMV, followed by FMI and FMSN. |
| Discovery response station entries (12 bytes) and network entries (13 bytes) | [bpetrikovics/hpavcli `interface.py` and `model.py`](https://github.com/bpetrikovics/hpavcli); public specification | Counted record parsing with truncation checks. Network offset uses the two-byte big-endian `H` layout in the MIT reference. Live discovery succeeded against the first BCM60500 target. |
| Broadcom OUI `00:1f:84`, Mediaxtream EtherType `0x8912`, network-info MMTYPE `0xa028`, request bytes `01 00 <mode>`, 19-byte response records | [bpetrikovics/hpavcli `model.py` and `interface.py`](https://github.com/bpetrikovics/hpavcli) | The response body prefix remains opaque. The 19-byte layout and offsets were validated against live TL-PA9020P / BCM60500 responses. The final fields correspond to access-network state, coordinating-neighbor-network count, and powerline status. |
| `CM_NW_STATS` entries (peer MAC plus two little-endian 16-bit rates) | [bpetrikovics/hpavcli `model.py` and `interface.py`](https://github.com/bpetrikovics/hpavcli) | Used for bidirectional PHY-rate output. Live responses validate the byte order and directional interpretation. |
| `jbit/powerline` architecture and Linux transport coverage | [README](https://github.com/jbit/powerline), [HomePlug modules](https://github.com/jbit/powerline/tree/main/src/homeplug) | Design reference for separating protocol handling from Linux raw-Ethernet transport. No code is reused. |
| `bpetrikovics/hpavcli` Broadcom coverage | [README](https://github.com/bpetrikovics/hpavcli), [source](https://github.com/bpetrikovics/hpavcli) | MIT interoperability reference for Broadcom network-info and standard network-stats layouts. No code is reused. |

## Validation coverage

| Capability | Current kwoopla evidence |
| --- | --- |
| Linux raw Ethernet transport | Built natively on x86_64 Linux; cross-built for `aarch64-unknown-linux-musl`; executed on kooWrt AX3600 |
| Standard HomePlug discovery/capability | Unit fixtures plus live TL-PA9020P acceptance |
| Broadcom `0x8912` network info | 19-byte layout and query path live-validated on TL-PA9020P / BCM60500 |
| Average PHY rates | Both directions live-validated against raw `CM_NW_STATS` responses |
