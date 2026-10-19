use super::{discover, query};
use anyhow::{Context, Result, bail, ensure};
use clap::{Subcommand, ValueEnum};
use etherparse::Ethernet2Slice;
use kwoopla::{
    APPLY_CONFIRM, BROADCOM_OUI, ETHERTYPE_HPAV, ETHERTYPE_MEDIAXTREAM, GET_PARAM_CONFIRM,
    MacAddress, ManagementMessage, PARAM_POWER_STANDBY, PARAM_POWER_STANDBY_AUX, PowerSavingState,
    SET_PARAM_ALT_CONFIRM, SET_PARAM_CONFIRM, Transport, encode_apply, encode_get_parameter,
    encode_set_parameter, parse_parameter_value, parse_station_capabilities, strip_vendor_prefix,
};
use serde::Serialize;
use std::time::Duration;

#[derive(Subcommand, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum PowerSavingOperation {
    /// Inspect each adapter or one selected adapter. No settings are changed.
    Get {
        #[arg(long)]
        device: Option<String>,
    },
    /// Change a selected adapter, then verify its persisted setting by reading it back.
    Set {
        #[arg(long)]
        device: String,
        #[arg(value_enum)]
        mode: PowerSavingMode,
    },
}

#[derive(Clone, Copy, ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum PowerSavingMode {
    On,
    Off,
}

impl PowerSavingMode {
    fn enabled(self) -> bool {
        matches!(self, Self::On)
    }
}

#[derive(Serialize)]
struct DeviceState {
    device_mac: MacAddress,
    state: Option<PowerSavingState>,
    status: &'static str,
}

#[derive(Serialize)]
struct GetOutput<'a> {
    schema_version: u32,
    command: &'static str,
    operation: &'static str,
    interface: &'a str,
    devices: Vec<DeviceState>,
}

#[derive(Serialize)]
struct SetOutput<'a> {
    schema_version: u32,
    command: &'static str,
    operation: &'static str,
    interface: &'a str,
    device_mac: MacAddress,
    previous: PowerSavingState,
    current: PowerSavingState,
    changed: bool,
    verified: bool,
}

fn vendor_get<T: Transport>(
    transport: &mut T,
    device: MacAddress,
) -> Result<Option<PowerSavingState>> {
    const SEQ: u8 = 7;
    let request = encode_get_parameter(SEQ, PARAM_POWER_STANDBY);
    let body = query(
        transport,
        device,
        ETHERTYPE_MEDIAXTREAM,
        &request,
        GET_PARAM_CONFIRM,
    )
    .with_context(|| format!("reading power-saving setting from {device}"))?;
    body.map(|body| {
        let value = parse_parameter_value(&body, SEQ)?;
        PowerSavingState::from_bytes(value)
    })
    .transpose()
    .context("parsing Broadcom power-saving response")
}

fn is_broadcom<T: Transport>(transport: &mut T, device: MacAddress) -> Result<Option<bool>> {
    let request = ManagementMessage::encode(1, 0x6034, &[]);
    let body = query(transport, device, ETHERTYPE_HPAV, &request, 0x6035)
        .with_context(|| format!("reading capabilities from {device}"))?;
    Ok(body
        .and_then(|data| parse_station_capabilities(&data).ok())
        .map(|cap| cap.oui == BROADCOM_OUI))
}

/// A confirmed ACK must come from the selected device, on the correct EtherType,
/// with the requested OUI, sequence and MMTYPE. Unrelated status frames do not count.
fn vendor_ack<T: Transport>(
    transport: &mut T,
    device: MacAddress,
    sequence: u8,
    request: &[u8],
    confirmation: u16,
    alternate: Option<u16>,
) -> Result<()> {
    let frames = transport.exchange(
        device,
        ETHERTYPE_MEDIAXTREAM,
        request,
        Duration::from_millis(1800),
    )?;
    for raw in frames {
        let Ok(frame) = Ethernet2Slice::from_slice_without_fcs(&raw) else {
            continue;
        };
        if frame.ether_type().0 != ETHERTYPE_MEDIAXTREAM || MacAddress(frame.source()) != device {
            continue;
        }
        let Ok(message) = ManagementMessage::parse(frame.payload_slice()) else {
            continue;
        };
        if message.version != 2
            || !(message.mmtype == confirmation || alternate == Some(message.mmtype))
        {
            continue;
        }
        // The confirmations have no application payload; Ethernet padding may be zero-filled.
        if let Ok(payload) = strip_vendor_prefix(message.body, sequence) {
            ensure!(
                payload.iter().all(|byte| *byte == 0),
                "device {device} returned an unrecognized nonzero confirmation body"
            );
            return Ok(());
        }
    }
    bail!("device {device} did not acknowledge Mediaxtream 0x{confirmation:04x}")
}

fn change_state<T: Transport>(
    transport: &mut T,
    device: MacAddress,
    old: PowerSavingState,
    enabled: bool,
) -> Result<PowerSavingState> {
    if old.enabled == enabled {
        return Ok(old);
    }
    ensure!(
        old.standby_timeout_seconds != 0,
        "device {device} reports zero standby timeout; refusing to guess a replacement"
    );
    vendor_ack(
        transport,
        device,
        8,
        &encode_set_parameter(8, PARAM_POWER_STANDBY, &old.with_enabled(enabled)),
        SET_PARAM_CONFIRM,
        Some(SET_PARAM_ALT_CONFIRM),
    )
    .context("setting the standby enable bit")?;
    if !enabled {
        vendor_ack(
            transport,
            device,
            9,
            &encode_set_parameter(9, PARAM_POWER_STANDBY_AUX, &[0]),
            SET_PARAM_CONFIRM,
            Some(SET_PARAM_ALT_CONFIRM),
        )
        .context("clearing the standby companion parameter")?;
    }
    vendor_ack(
        transport,
        device,
        10,
        &encode_apply(10),
        APPLY_CONFIRM,
        None,
    )
    .context("applying the power-saving configuration")?;
    for _ in 0..2 {
        if let Some(current) = vendor_get(transport, device)? {
            if current.enabled == enabled
                && current.standby_timeout_seconds == old.standby_timeout_seconds
            {
                return Ok(current);
            }
        }
    }
    bail!(
        "configuration apply was acknowledged, but device {device} failed power-saving read-back verification"
    )
}

pub(super) fn run<T: Transport>(
    transport: &mut T,
    interface: &str,
    operation: PowerSavingOperation,
    json: bool,
) -> Result<String> {
    let (devices, _) =
        discover(transport).context("discovering adapters before power-saving access")?;
    match operation {
        PowerSavingOperation::Get { device } => {
            let target = device.map(|text| MacAddress::parse(&text)).transpose()?;
            if let Some(target) = target {
                ensure!(
                    devices.iter().any(|d| d.mac == target),
                    "device {target} was not discovered on {interface}"
                );
            }
            let mut states = Vec::new();
            for item in devices.iter().filter(|d| target.is_none_or(|t| d.mac == t)) {
                let (state, status) = match is_broadcom(transport, item.mac)? {
                    Some(true) => match vendor_get(transport, item.mac) {
                        Ok(Some(state)) => (Some(state), "available"),
                        Ok(None) => (None, "no_response"),
                        Err(error) => {
                            if matches!(
                                error.root_cause().downcast_ref::<kwoopla::Error>(),
                                Some(kwoopla::Error::Truncated(_) | kwoopla::Error::Invalid(_))
                            ) {
                                (None, "malformed_response")
                            } else {
                                return Err(error.context(format!("reading device {}", item.mac)));
                            }
                        }
                    },
                    Some(false) => (None, "not_broadcom"),
                    None => (None, "capabilities_unavailable"),
                };
                states.push(DeviceState {
                    device_mac: item.mac,
                    state,
                    status,
                });
            }
            let output = GetOutput {
                schema_version: 1,
                command: "power_saving",
                operation: "get",
                interface,
                devices: states,
            };
            if !json {
                for entry in &output.devices {
                    if let Some(state) = entry.state {
                        println!(
                            "Device {}: {}; standby {} seconds",
                            entry.device_mac,
                            if state.enabled { "on" } else { "off" },
                            state.standby_timeout_seconds
                        );
                    } else {
                        println!("Device {}: {}", entry.device_mac, entry.status);
                    }
                }
            }
            Ok(serde_json::to_string_pretty(&output)?)
        }
        PowerSavingOperation::Set { device, mode } => {
            let device = MacAddress::parse(&device)?;
            ensure!(
                devices.iter().any(|d| d.mac == device),
                "device {device} was not discovered on {interface}"
            );
            ensure!(
                is_broadcom(transport, device)? == Some(true),
                "device {device} is not a confirmed Broadcom adapter"
            );
            let previous = vendor_get(transport, device)?.ok_or_else(|| {
                anyhow::anyhow!(
                    "device {device} did not answer power-saving Get Parameter; refusing to write"
                )
            })?;
            let current = change_state(transport, device, previous, mode.enabled())?;
            let output = SetOutput {
                schema_version: 1,
                command: "power_saving",
                operation: "set",
                interface,
                device_mac: device,
                previous,
                current,
                changed: previous != current,
                verified: true,
            };
            if !json {
                println!(
                    "Device {device}: {} -> {} (standby {} seconds, verified)",
                    if previous.enabled { "on" } else { "off" },
                    if current.enabled { "on" } else { "off" },
                    current.standby_timeout_seconds
                );
            }
            Ok(serde_json::to_string_pretty(&output)?)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    struct MockTransport {
        responses: VecDeque<Vec<Vec<u8>>>,
        requests: Vec<Vec<u8>>,
    }

    impl Transport for MockTransport {
        fn exchange(
            &mut self,
            _: MacAddress,
            _: u16,
            request: &[u8],
            _: Duration,
        ) -> Result<Vec<Vec<u8>>, kwoopla::Error> {
            self.requests.push(request.to_vec());
            Ok(self.responses.pop_front().unwrap_or_default())
        }
    }

    fn response(mac: MacAddress, mmtype: u16, sequence: u8, extra: &[u8]) -> Vec<u8> {
        let mut frame = vec![0xff; 6];
        frame.extend_from_slice(&mac.0);
        frame.extend_from_slice(&ETHERTYPE_MEDIAXTREAM.to_be_bytes());
        let mut body = [0, 0x1f, 0x84, sequence].to_vec();
        body.extend_from_slice(extra);
        frame.extend_from_slice(&ManagementMessage::encode(2, mmtype, &body));
        frame
    }

    fn mock(mac: MacAddress, rows: &[(u16, u8, &[u8])]) -> MockTransport {
        MockTransport {
            responses: rows
                .iter()
                .map(|(mmtype, sequence, extra)| vec![response(mac, *mmtype, *sequence, extra)])
                .collect(),
            requests: Vec::new(),
        }
    }

    #[test]
    fn disable_emits_companion_and_apply_then_requires_matching_readback() {
        let mac = MacAddress([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);
        let mut transport = mock(
            mac,
            &[
                (SET_PARAM_CONFIRM, 8, &[]),
                (SET_PARAM_CONFIRM, 9, &[]),
                (APPLY_CONFIRM, 10, &[]),
                (GET_PARAM_CONFIRM, 7, &[2, 1, 0, 0x2c, 0x01]),
            ],
        );
        let previous = PowerSavingState {
            enabled: true,
            standby_timeout_seconds: 300,
        };
        let current = change_state(&mut transport, mac, previous, false).unwrap();
        assert!(!current.enabled);
        assert_eq!(current.standby_timeout_seconds, 300);
        assert_eq!(transport.requests.len(), 4);
        assert_eq!(
            transport.requests[0],
            encode_set_parameter(8, PARAM_POWER_STANDBY, &[0x2c, 0x01])
        );
        assert_eq!(
            transport.requests[1],
            encode_set_parameter(9, PARAM_POWER_STANDBY_AUX, &[0])
        );
        assert_eq!(transport.requests[2], encode_apply(10));
        assert_eq!(
            transport.requests[3],
            encode_get_parameter(7, PARAM_POWER_STANDBY)
        );
    }

    #[test]
    fn unverified_confirmation_fails_closed_before_apply() {
        let mac = MacAddress([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);
        let old = PowerSavingState {
            enabled: true,
            standby_timeout_seconds: 300,
        };
        let mut transport = mock(mac, &[(SET_PARAM_CONFIRM, 99, &[])]);
        assert!(change_state(&mut transport, mac, old, false).is_err());
        assert_eq!(transport.requests.len(), 1);

        let mut transport = mock(mac, &[]);
        let unknown_timeout = PowerSavingState {
            enabled: true,
            standby_timeout_seconds: 0,
        };
        assert!(change_state(&mut transport, mac, unknown_timeout, false).is_err());
        assert!(transport.requests.is_empty());
    }

    #[test]
    fn enable_preserves_timeout_without_companion_write() {
        let mac = MacAddress([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);
        let mut transport = mock(
            mac,
            &[
                (SET_PARAM_ALT_CONFIRM, 8, &[]),
                (APPLY_CONFIRM, 10, &[]),
                (GET_PARAM_CONFIRM, 7, &[2, 1, 0, 0x2c, 0x81]),
            ],
        );
        let previous = PowerSavingState {
            enabled: false,
            standby_timeout_seconds: 300,
        };
        assert!(
            change_state(&mut transport, mac, previous, true)
                .unwrap()
                .enabled
        );
        assert_eq!(transport.requests.len(), 3);
        assert_eq!(
            transport.requests[0],
            encode_set_parameter(8, PARAM_POWER_STANDBY, &[0x2c, 0x81])
        );
        assert_eq!(transport.requests[1], encode_apply(10));
    }
}
