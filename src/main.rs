use clap::{Parser, Subcommand};
use etherparse::Ethernet2Slice;
use kwoopla::{
    BROADCOM_OUI, DiscoveredNetwork, DiscoveredStation, ETHERTYPE_HPAV, ETHERTYPE_MEDIAXTREAM,
    Error, LinuxTransport, MacAddress, ManagementMessage, PhyRate, StationCapabilities, Transport,
    parse_broadcom_network_info, parse_discover_list, parse_network_stats,
    parse_station_capabilities,
};
use serde::Serialize;
use std::time::Duration;

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

#[derive(Subcommand, Clone, Copy)]
enum Command {
    Discover,
    Info,
    Rates,
}

#[derive(Serialize)]
struct DiscoverOutput {
    schema_version: u32,
    command: &'static str,
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
    command: &'static str,
    interface: String,
    devices: Vec<DeviceInfo>,
}

#[derive(Serialize)]
struct DeviceInfo {
    mac: MacAddress,
    capabilities: Option<StationCapabilities>,
    networks: Option<Vec<kwoopla::BroadcomNetwork>>,
    vendor_info_status: &'static str,
}

#[derive(Serialize)]
struct RatesOutput {
    schema_version: u32,
    command: &'static str,
    interface: String,
    rates: Vec<DeviceRates>,
}

#[derive(Serialize)]
struct DeviceRates {
    device_mac: MacAddress,
    peers: Vec<PhyRate>,
    status: &'static str,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("kwoopla: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Error> {
    let args = Args::parse();
    let interface = args.interface.as_deref().ok_or(Error::Invalid(
        "an interface is required; pass -i/--interface",
    ))?;
    let mut transport = LinuxTransport::new(interface)?;
    let (output, result_count) = match args.command {
        Command::Discover => {
            let (devices, networks) = discover(&mut transport)?;
            let result_count = devices.len();
            (
                serde_json::to_string_pretty(&DiscoverOutput {
                    schema_version: 1,
                    command: "discover",
                    interface: interface.to_owned(),
                    devices,
                    networks,
                })
                .unwrap(),
                result_count,
            )
        }
        Command::Info => {
            let (devices, _) = discover(&mut transport)?;
            let mut info = Vec::new();
            for device in devices {
                let capabilities = query(
                    &mut transport,
                    device.mac,
                    ETHERTYPE_HPAV,
                    &ManagementMessage::encode(1, 0x6034, &[]),
                    0x6035,
                )
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
                        ) {
                            Some(body) => match parse_broadcom_network_info(&body) {
                                Ok(value) => (Some(value), "available"),
                                Err(_) => (None, "malformed_response"),
                            },
                            None => (None, "no_response"),
                        }
                    } else if capabilities.is_some() {
                        (None, "not_broadcom")
                    } else {
                        (None, "capabilities_unavailable")
                    };
                info.push(DeviceInfo {
                    mac: device.mac,
                    capabilities,
                    networks,
                    vendor_info_status,
                });
            }
            let result_count = info.len();
            let output = serde_json::to_string_pretty(&InfoOutput {
                schema_version: 1,
                command: "info",
                interface: interface.to_owned(),
                devices: info,
            })
            .unwrap();
            (output, result_count)
        }
        Command::Rates => {
            let (devices, _) = discover(&mut transport)?;
            let mut rates = Vec::new();
            for device in devices {
                let body = query(
                    &mut transport,
                    device.mac,
                    ETHERTYPE_HPAV,
                    &ManagementMessage::encode(2, 0x6048, &[]),
                    0x6049,
                );
                let (peers, status) = match body {
                    Some(body) => match parse_network_stats(&body) {
                        Ok(peers) => (peers, "available"),
                        Err(_) => (Vec::new(), "malformed_response"),
                    },
                    None => (Vec::new(), "no_response"),
                };
                rates.push(DeviceRates {
                    device_mac: device.mac,
                    peers,
                    status,
                });
            }
            let result_count = rates.len();
            (
                serde_json::to_string_pretty(&RatesOutput {
                    schema_version: 1,
                    command: "rates",
                    interface: interface.to_owned(),
                    rates,
                })
                .unwrap(),
                result_count,
            )
        }
    };
    if args.json {
        println!("{output}");
    } else {
        println!(
            "{}: {} result(s) on {}",
            match args.command {
                Command::Discover => "discover",
                Command::Info => "info",
                Command::Rates => "rates",
            },
            result_count,
            interface
        );
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
        let (stations, nets) = parse_discover_list(message.body)?;
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
) -> Option<Vec<u8>> {
    let frames = transport
        .exchange(device, ethertype, request, Duration::from_millis(500))
        .ok()?;
    for bytes in frames {
        let frame = Ethernet2Slice::from_slice_without_fcs(&bytes).ok()?;
        if MacAddress(frame.source()) != device || frame.ether_type().0 != ethertype {
            continue;
        }
        let message = ManagementMessage::parse(frame.payload_slice()).ok()?;
        if message.mmtype == response_type {
            return Some(message.body.to_vec());
        }
    }
    None
}
