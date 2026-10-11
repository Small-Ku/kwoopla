use super::{DiscoverOutput, InfoOutput, RatesOutput};

pub(super) fn render_discover(output: &DiscoverOutput) {
    println!("HomePlug discovery on {}", output.interface);
    if output.devices.is_empty() {
        println!("No HomePlug devices were discovered.");
        return;
    }
    for device in &output.devices {
        println!("Device {}", device.mac);
        if device.stations.is_empty() {
            println!("  Stations: none reported");
        } else {
            println!("  Stations: {}", device.stations.len());
            for station in &device.stations {
                println!(
                    "    {}{}",
                    station.mac,
                    if station.same_network {
                        " (same network)"
                    } else {
                        ""
                    }
                );
            }
        }
    }
    for network in &output.networks {
        println!(
            "Network {} (SNID {}, access: {}, hybrid mode: {}, coordinator status: {})",
            network.network_id,
            network.short_network_id,
            network.access_network,
            network.hybrid_mode,
            network.coordinator_status
        );
    }
}

pub(super) fn render_info(output: &InfoOutput) {
    println!("HomePlug device information on {}", output.interface);
    if output.devices.is_empty() {
        println!("No HomePlug devices were discovered.");
        return;
    }
    for device in &output.devices {
        println!("Device {}", device.mac);
        if let Some(capabilities) = &device.capabilities {
            let oui = capabilities.oui.map(|byte| format!("{byte:02x}")).join(":");
            println!("  AV version: {}; OUI: {oui}", capabilities.av_version);
        } else {
            println!("  Capabilities: unavailable");
        }
        if let Some(networks) = &device.networks {
            for network in networks {
                println!(
                    "  Network {} (SNID {}): role {}; coordinator {}; access {}; neighbor networks {}; status {}",
                    network.network_id,
                    network.short_network_id,
                    role_label(network.role),
                    network.coordinator_mac,
                    network.access_network,
                    network.coordinating_neighbor_network_count,
                    powerline_status_label(network.powerline_status)
                );
            }
        } else {
            println!(
                "  Broadcom network info: {}",
                device.vendor_info_status.as_str()
            );
        }
    }
}

fn role_label(role: u8) -> String {
    let label = match role {
        0 => "unassociated station",
        1 => "unassociated coordinator",
        2 => "station",
        3 => "coordinator",
        4 => "backup coordinator",
        _ => return format!("unknown ({role})"),
    };
    format!("{label} ({role})")
}

fn powerline_status_label(status: u8) -> String {
    let label = match status {
        0 => "joined",
        1 => "not joined (NMK available)",
        2 => "not joined (no NMK)",
        _ => return format!("unknown ({status})"),
    };
    format!("{label} ({status})")
}

pub(super) fn render_rates(output: &RatesOutput) {
    println!("HomePlug rates on {}", output.interface);
    if output.rates.is_empty() {
        println!("No HomePlug devices were discovered.");
        return;
    }
    for device in &output.rates {
        println!("Device {}: {}", device.device_mac, device.status.as_str());
        for peer in &device.peers {
            println!(
                "  Peer {}: tx {} Mbps, rx {} Mbps",
                peer.peer_mac, peer.transmit_mbps, peer.receive_mbps
            );
        }
    }
}
