mod render;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use etherparse::Ethernet2Slice;
#[cfg(target_os = "linux")]
use kwoopla::LinuxTransport as PlatformTransport;
#[cfg(not(any(target_os = "linux", windows)))]
use kwoopla::LinuxTransport as PlatformTransport;
#[cfg(windows)]
use kwoopla::WindowsTransport as PlatformTransport;
use kwoopla::{
    BROADCOM_OUI, DiscoveredNetwork, DiscoveredStation, ETHERTYPE_HPAV, ETHERTYPE_MEDIAXTREAM,
    Error, MacAddress, ManagementMessage, PhyRate, StationCapabilities, Transport,
    parse_broadcom_network_info, parse_discover_list, parse_network_stats,
    parse_station_capabilities,
};
use render::{render_discover, render_info, render_rates};
use serde::Serialize;
use std::time::Duration;

const SCHEMA_VERSION: u32 = 1;

#[derive(Parser)]
#[command(
    name = "kwoopla",
    version,
    about = "HomePlug AV/AV2 discovery and diagnostics"
)]
struct Args {
    #[arg(
        short,
        long,
        global = true,
        help = "Network interface for raw Ethernet (required)"
    )]
    interface: Option<String>,
    #[arg(long, global = true, help = "Emit versioned JSON")]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum Command {
    Discover,
    Info,
    Rates,
}

#[derive(Serialize)]
struct DiscoverOutput {
    schema_version: u32,
    command: Command,
    interface: String,
    devices: Vec<DeviceDiscovery>,
    networks: Vec<DiscoveredNetwork>,
}

#[derive(Serialize)]
struct DeviceDiscovery {
    mac: MacAddress,
    stations: Vec<DiscoveredStation>,
}

#[derive(Serialize)]
struct InfoOutput {
    schema_version: u32,
    command: Command,
    interface: String,
    devices: Vec<DeviceInfo>,
}

#[derive(Serialize)]
struct DeviceInfo {
    mac: MacAddress,
    capabilities: Option<StationCapabilities>,
    networks: Option<Vec<kwoopla::BroadcomNetwork>>,
    vendor_info_status: VendorInfoStatus,
}

#[derive(Serialize)]
struct RatesOutput {
    schema_version: u32,
    command: Command,
    interface: String,
    rates: Vec<DeviceRates>,
}

#[derive(Serialize)]
struct DeviceRates {
    device_mac: MacAddress,
    peers: Vec<PhyRate>,
    status: RateStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum VendorInfoStatus {
    Available,
    MalformedResponse,
    NoResponse,
    NotBroadcom,
    CapabilitiesUnavailable,
}

impl VendorInfoStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::MalformedResponse => "malformed_response",
            Self::NoResponse => "no_response",
            Self::NotBroadcom => "not_broadcom",
            Self::CapabilitiesUnavailable => "capabilities_unavailable",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum RateStatus {
    Available,
    MalformedResponse,
    NoResponse,
}

impl RateStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::MalformedResponse => "malformed_response",
            Self::NoResponse => "no_response",
        }
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("kwoopla: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = Args::parse();
    let interface = args
        .interface
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("an interface is required; pass -i/--interface"))?;
    let mut transport = PlatformTransport::new(interface)
        .with_context(|| format!("opening selected interface {interface}"))?;
    let output = match args.command {
        Command::Discover => {
            let (devices, networks) = discover(&mut transport).context("performing discovery")?;
            let result = DiscoverOutput {
                schema_version: SCHEMA_VERSION,
                command: Command::Discover,
                interface: interface.to_owned(),
                devices,
                networks,
            };
            if !args.json {
                render_discover(&result);
            }
            serde_json::to_string_pretty(&result).context("serializing discover JSON output")?
        }
        Command::Info => {
            let (devices, _) = discover(&mut transport).context("performing discovery")?;
            let mut info = Vec::new();
            for device in devices {
                let capabilities = query(
                    &mut transport,
                    device.mac,
                    ETHERTYPE_HPAV,
                    &ManagementMessage::encode(1, 0x6034, &[]),
                    0x6035,
                )
                .with_context(|| format!("querying capabilities for {}", device.mac))?
                .and_then(|body| parse_station_capabilities(&body).ok());
                let (networks, vendor_info_status) =
                    if capabilities.as_ref().is_some_and(|c| c.oui == BROADCOM_OUI) {
                        let request = kwoopla::encode_broadcom_network_info_request(true);
                        match query(
                            &mut transport,
                            device.mac,
                            ETHERTYPE_MEDIAXTREAM,
                            &request,
                            0xa029,
                        )
                        .with_context(|| {
                            format!("querying vendor information for {}", device.mac)
                        })? {
                            Some(body) => match parse_broadcom_network_info(&body) {
                                Ok(value) => (Some(value), VendorInfoStatus::Available),
                                Err(_) => (None, VendorInfoStatus::MalformedResponse),
                            },
                            None => (None, VendorInfoStatus::NoResponse),
                        }
                    } else if capabilities.is_some() {
                        (None, VendorInfoStatus::NotBroadcom)
                    } else {
                        (None, VendorInfoStatus::CapabilitiesUnavailable)
                    };
                info.push(DeviceInfo {
                    mac: device.mac,
                    capabilities,
                    networks,
                    vendor_info_status,
                });
            }
            let result = InfoOutput {
                schema_version: SCHEMA_VERSION,
                command: Command::Info,
                interface: interface.to_owned(),
                devices: info,
            };
            if !args.json {
                render_info(&result);
            }
            serde_json::to_string_pretty(&result).context("serializing info JSON output")?
        }
        Command::Rates => {
            let (devices, _) = discover(&mut transport).context("performing discovery")?;
            let mut rates = Vec::new();
            for device in devices {
                let body = query(
                    &mut transport,
                    device.mac,
                    ETHERTYPE_HPAV,
                    &ManagementMessage::encode(2, 0x6048, &[]),
                    0x6049,
                )
                .with_context(|| format!("querying rates for {}", device.mac))?;
                let (peers, status) = match body {
                    Some(body) => match parse_network_stats(&body) {
                        Ok(peers) => (peers, RateStatus::Available),
                        Err(_) => (Vec::new(), RateStatus::MalformedResponse),
                    },
                    None => (Vec::new(), RateStatus::NoResponse),
                };
                rates.push(DeviceRates {
                    device_mac: device.mac,
                    peers,
                    status,
                });
            }
            let result = RatesOutput {
                schema_version: SCHEMA_VERSION,
                command: Command::Rates,
                interface: interface.to_owned(),
                rates,
            };
            if !args.json {
                render_rates(&result);
            }
            serde_json::to_string_pretty(&result).context("serializing rates JSON output")?
        }
    };
    if args.json {
        println!("{output}");
    }
    Ok(())
}

fn discover<T: Transport>(
    transport: &mut T,
) -> Result<(Vec<DeviceDiscovery>, Vec<DiscoveredNetwork>), Error> {
    let request = ManagementMessage::encode(1, 0x0014, &[]);
    let responses = transport.exchange(
        MacAddress::BROADCAST,
        ETHERTYPE_HPAV,
        &request,
        Duration::from_millis(500),
    )?;
    let mut devices = Vec::new();
    let mut networks = Vec::new();
    for bytes in responses {
        let Ok(frame) = Ethernet2Slice::from_slice_without_fcs(&bytes) else {
            continue;
        };
        let Ok(message) = ManagementMessage::parse(frame.payload_slice()) else {
            continue;
        };
        if frame.ether_type().0 != ETHERTYPE_HPAV || message.mmtype != 0x0015 {
            continue;
        }
        let Ok((stations, nets)) = parse_discover_list(message.body) else {
            continue;
        };
        devices.push(DeviceDiscovery {
            mac: MacAddress(frame.source()),
            stations,
        });
        networks.extend(nets);
    }
    devices.sort_by_key(|d| d.mac);
    devices.dedup_by_key(|d| d.mac);
    Ok((devices, networks))
}

fn query<T: Transport>(
    transport: &mut T,
    device: MacAddress,
    ethertype: u16,
    request: &[u8],
    response_type: u16,
) -> Result<Option<Vec<u8>>, Error> {
    let frames = transport.exchange(device, ethertype, request, Duration::from_millis(500))?;
    for bytes in frames {
        let Ok(frame) = Ethernet2Slice::from_slice_without_fcs(&bytes) else {
            continue;
        };
        if MacAddress(frame.source()) != device || frame.ether_type().0 != ethertype {
            continue;
        }
        let Ok(message) = ManagementMessage::parse(frame.payload_slice()) else {
            continue;
        };
        if message.mmtype == response_type {
            return Ok(Some(message.body.to_vec()));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::{Command, DiscoverOutput, RateStatus, SCHEMA_VERSION, VendorInfoStatus, query};
    use kwoopla::{Error, MacAddress, ManagementMessage, Transport};
    use std::time::Duration;

    struct Frames(Vec<Vec<u8>>);

    impl Transport for Frames {
        fn exchange(
            &mut self,
            _: MacAddress,
            _: u16,
            _: &[u8],
            _: Duration,
        ) -> Result<Vec<Vec<u8>>, Error> {
            Ok(std::mem::take(&mut self.0))
        }
    }

    #[test]
    fn query_skips_malformed_and_unrelated_frames_before_valid_response() {
        let device = MacAddress([0, 1, 2, 3, 4, 5]);
        let other = MacAddress([6, 7, 8, 9, 10, 11]);
        let mut valid = vec![0xff; 6];
        valid.extend_from_slice(&device.0);
        valid.extend_from_slice(&0x88e1_u16.to_be_bytes());
        valid.extend_from_slice(&ManagementMessage::encode(1, 0x6049, &[0xaa, 0xbb]));
        let mut wrong_source = valid.clone();
        wrong_source[6..12].copy_from_slice(&other.0);
        let mut malformed_mme = vec![0xff; 6];
        malformed_mme.extend_from_slice(&device.0);
        malformed_mme.extend_from_slice(&0x88e1_u16.to_be_bytes());
        malformed_mme.extend_from_slice(&[3, 0, 0, 0, 0]);
        let mut frames = Frames(vec![vec![1, 2, 3], wrong_source, malformed_mme, valid]);

        let response = query(&mut frames, device, 0x88e1, &[0], 0x6049).unwrap();
        assert_eq!(response, Some(vec![0xaa, 0xbb]));
    }

    #[test]
    fn status_values_serialize_to_the_existing_schema_strings() {
        let vendor_cases = [
            (VendorInfoStatus::Available, "available"),
            (VendorInfoStatus::MalformedResponse, "malformed_response"),
            (VendorInfoStatus::NoResponse, "no_response"),
            (VendorInfoStatus::NotBroadcom, "not_broadcom"),
            (
                VendorInfoStatus::CapabilitiesUnavailable,
                "capabilities_unavailable",
            ),
        ];
        let rate_cases = [
            (RateStatus::Available, "available"),
            (RateStatus::MalformedResponse, "malformed_response"),
            (RateStatus::NoResponse, "no_response"),
        ];

        for (status, expected) in vendor_cases {
            assert_eq!(
                serde_json::to_string(&status).unwrap(),
                format!("\"{expected}\"")
            );
        }
        for (status, expected) in rate_cases {
            assert_eq!(
                serde_json::to_string(&status).unwrap(),
                format!("\"{expected}\"")
            );
        }
    }

    #[test]
    fn output_serializes_the_versioned_schema_field() {
        let output = DiscoverOutput {
            schema_version: SCHEMA_VERSION,
            command: Command::Discover,
            interface: "eth-test".to_owned(),
            devices: Vec::new(),
            networks: Vec::new(),
        };
        let json = serde_json::to_value(output).unwrap();

        assert_eq!(json["schema_version"], 1);
        assert_eq!(json["command"], "discover");
    }
}
