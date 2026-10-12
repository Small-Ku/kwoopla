# Protocol provenance

This document lists sources that directly informed the current protocol implementation. No third-party implementation code has been copied into kwoopla.

| Protocol detail | Source | Use |
| --- | --- | --- |
| HomePlug AV EtherType `0x88e1`, management framing, and standard message types | Public [HomePlug AV 1.1 specification](https://docbox.etsi.org/Reference/homeplug_av11/homeplug_av11_specification_final_public.pdf); [jbit/powerline](https://github.com/jbit/powerline) | `CC_DISCOVER_LIST`, `CM_STA_CAP`, and `CM_NW_STATS` framing and parsing |
| AV 1.1/2.0 management header and little-endian MMTYPE | HomePlug AV 1.1 specification; [jbit/powerline message definitions](https://github.com/jbit/powerline/tree/main/src/homeplug) | Management message encoding and decoding |
| Discovery station and network record layouts | HomePlug AV 1.1 specification; [bpetrikovics/hpavcli](https://github.com/bpetrikovics/hpavcli) | Counted record parsing and bounds checks |
| Broadcom OUI `00:1f:84`, Mediaxtream EtherType `0x8912`, and network-info layout | [bpetrikovics/hpavcli](https://github.com/bpetrikovics/hpavcli) | Broadcom network-info queries; the 19-byte response layout was checked against TL-PA9020P responses |
| `CM_NW_STATS` peer MAC and little-endian PHY-rate fields | [bpetrikovics/hpavcli](https://github.com/bpetrikovics/hpavcli) | Bidirectional PHY-rate output; byte order was checked against live responses |

## Current checks

| Path | Evidence |
| --- | --- |
| Linux transport | Native x86_64 build; aarch64-musl build; runtime check on AX3600 |
| HomePlug discovery and station capabilities | Unit fixtures and TL-PA9020P runtime checks |
| Broadcom network information | TL-PA9020P / BCM60500 runtime checks |
| PHY rates | Both directions checked against raw `CM_NW_STATS` responses |

The BSD-2-Clause `jbit/powerline` project is used as an architecture and framing reference. The MIT `bpetrikovics/hpavcli` project is used as an interoperability reference for Broadcom and network-stat layouts.
