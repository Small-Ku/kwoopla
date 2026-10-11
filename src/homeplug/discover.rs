use crate::{Error, MacAddress};
use serde::Serialize;
use zerocopy::byteorder::big_endian::U16 as BeU16;
use zerocopy::{FromBytes, Immutable, KnownLayout, Unaligned};

#[derive(FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
struct DiscoveredStationWire {
    mac: [u8; 6],
    terminal_equipment_id: u8,
    same_network: u8,
    snid_access: u8,
    capabilities: u8,
    signal_level: u8,
    average_ble: u8,
}

#[derive(FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
struct DiscoveredNetworkWire {
    network_id: [u8; 7],
    snid_access: u8,
    hybrid_mode: u8,
    beacon_slots: u8,
    coordinator_status: u8,
    offset: BeU16,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DiscoveredStation {
    pub mac: MacAddress,
    pub terminal_equipment_id: u8,
    pub same_network: bool,
    pub short_network_id: u8,
    pub access_network: bool,
    pub capabilities: u8,
    pub signal_level: u8,
    pub average_ble: u8,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DiscoveredNetwork {
    pub network_id: String,
    pub short_network_id: u8,
    pub access_network: bool,
    pub hybrid_mode: u8,
    pub beacon_slots: u8,
    pub coordinator_status: u8,
    pub offset: u16,
}

pub fn parse_discover_list(
    body: &[u8],
) -> Result<(Vec<DiscoveredStation>, Vec<DiscoveredNetwork>), Error> {
    let (station_count, body) = body
        .split_first()
        .ok_or(Error::Truncated("discover-list station count"))?;
    let (station_records, body) =
        <[DiscoveredStationWire]>::ref_from_prefix_with_elems(body, *station_count as usize)
            .map_err(|_| Error::Truncated("discover-list station records"))?;
    let (network_count, body) = body
        .split_first()
        .ok_or(Error::Truncated("discover-list network count"))?;
    let (network_records, _) =
        <[DiscoveredNetworkWire]>::ref_from_prefix_with_elems(body, *network_count as usize)
            .map_err(|_| Error::Truncated("discover-list network records"))?;

    let stations = station_records
        .iter()
        .map(|record| DiscoveredStation {
            mac: MacAddress(record.mac),
            terminal_equipment_id: record.terminal_equipment_id,
            same_network: record.same_network != 0,
            short_network_id: record.snid_access & 0x0f,
            access_network: record.snid_access & 0xf0 != 0,
            capabilities: record.capabilities,
            signal_level: record.signal_level,
            average_ble: record.average_ble,
        })
        .collect();
    let networks = network_records
        .iter()
        .map(|record| DiscoveredNetwork {
            network_id: record
                .network_id
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
            short_network_id: record.snid_access & 0x0f,
            access_network: record.snid_access & 0xf0 != 0,
            hybrid_mode: record.hybrid_mode,
            beacon_slots: record.beacon_slots,
            coordinator_status: record.coordinator_status,
            offset: record.offset.get(),
        })
        .collect();
    Ok((stations, networks))
}

#[cfg(test)]
mod tests {
    use super::parse_discover_list;
    use crate::MacAddress;

    #[test]
    fn discover_list_fixture_and_truncation() {
        let fixture = [
            0x01, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x07, 0x01, 0x23, 0x04, 0x0a, 0x02, 0x01,
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x04, 0x05,
        ];
        let (stations, networks) = parse_discover_list(&fixture).unwrap();
        assert_eq!(
            stations[0].mac,
            MacAddress([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff])
        );
        assert_eq!(networks[0].network_id, "01020304050607");
        assert_eq!(networks[0].offset, 0x0405);
        assert!(parse_discover_list(&fixture[..fixture.len() - 1]).is_err());
        assert!(parse_discover_list(&[]).is_err());
    }
}
