# Project guidance

- Keep protocol code separate from platform transport code.
- Keep unrelated management commands read-only. The explicit Power Saving exception permits only per-device changes with preflight read, preserved standby timeout, acknowledged writes and apply, and verified readback. Do not introduce key management, restart, or factory reset.
- Require an explicit Ethernet interface. Do not select one automatically.
- Treat TP-Link TL-PA9020P / Broadcom BCM60500 as the first tested device. Mark behavior that lacks device evidence.
- Use HomePlug specifications and compatible BSD/MIT references. Do not copy code from license-incompatible sources.
- Keep required licenses and attribution when reusing third-party code. The project license is `MIT OR Apache-2.0`; do not change it without a separate decision.
- Keep JSON output typed and versioned. Add fixed byte fixtures and malformed-input tests when changing protocol parsing.
- Do not add broad vendor support, public release, or crate publication without a separate decision.

## Build and checks

- `mise run build` uses the pinned nightly Cranelift backend for workspace code, LLVM for dependencies, and Mr Boxington.
- Use stable LLVM for `check`, Clippy, tests, release builds, and runtime tests. Check Linux `AF_PACKET`, Windows Npcap, and real-device behavior separately.
- Clippy rejects `unwrap()` and `expect()` outside tests.
- Keep the Npcap runtime as a Windows system dependency. mise manages the Npcap SDK; do not vendor it.
