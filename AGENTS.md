# Project guidance

- Keep the project independent of KooWrt and keep transport code separate from protocol code.
- Prefer small, synchronous, read-only steps. Do not add network mutation, NMK/DAK changes, restart, or factory reset without a separate scope decision.
- Require explicit interface selection for raw Ethernet commands; do not silently select an interface.
- Treat the TL-PA9020P / BCM60500 as the first live target. Mark behavior that lacks device evidence and avoid guessing undocumented fields.
- HomePlug AV/AV2 specifications and compatible BSD/MIT references may be used for protocol work. Do not derive implementation code from license-incompatible third-party source. Independent black-box observations may be used as interoperability evidence.
- If code is reused from another project, preserve its license and required attribution separately. Do not vendor third-party source without a deliberate decision.
- Keep JSON schemas typed and versioned. Add fixed byte fixtures, malformed-input tests, and output-shape tests when changing a protocol path.
- No KooWrt package, ubus, LuCI, broad vendor framework, public release, or publication without a separate scope decision.

## Host/runtime boundary

The protocol crate must continue to build and test on Windows. Linux `AF_PACKET`, future Windows Npcap transport, and real-device behavior must be validated independently; a build on one platform is not runtime acceptance for another.
