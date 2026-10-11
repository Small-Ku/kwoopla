use crate::{Error, MacAddress};
use serde::Serialize;
use zerocopy::byteorder::little_endian::U16 as LeU16;
use zerocopy::{FromBytes, Immutable, KnownLayout, Unaligned};

#[derive(FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
struct PhyRateWire {
    peer_mac: [u8; 6],
    transmit_mbps: LeU16,
    receive_mbps: LeU16,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PhyRate {
    pub peer_mac: MacAddress,
    pub transmit_mbps: u16,
    pub receive_mbps: u16,
}

pub fn parse_network_stats(body: &[u8]) -> Result<Vec<PhyRate>, Error> {
    let (count, body) = body
        .split_first()
        .ok_or(Error::Truncated("network-stats count"))?;
    let (records, _) = <[PhyRateWire]>::ref_from_prefix_with_elems(body, *count as usize)
        .map_err(|_| Error::Truncated("network-stats records"))?;
    Ok(records
        .iter()
        .map(|record| PhyRate {
            peer_mac: MacAddress(record.peer_mac),
            transmit_mbps: record.transmit_mbps.get(),
            receive_mbps: record.receive_mbps.get(),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::parse_network_stats;
    use crate::homeplug::ManagementMessage;
    use etherparse::Ethernet2Slice;

    // Static HomePlug Ethernet bytes: CM_NW_STATS.CNF with one peer at 1200/1100 Mbps.
    const RATE_FIXTURE: [u8; 32] = [
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0x88, 0xe1, 0x02,
        0x49, 0x60, 0x00, 0x00, 0x01, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xb0, 0x04, 0x4c, 0x04,
        0x00, 0x00,
    ];

    #[test]
    fn ethernet_homeplug_network_stats_fixture() {
        let frame = Ethernet2Slice::from_slice_without_fcs(&RATE_FIXTURE).unwrap();
        assert_eq!(frame.ether_type().0, 0x88e1);
        let message = ManagementMessage::parse(frame.payload_slice()).unwrap();
        assert_eq!(message.mmtype, 0x6049);
        let rates = parse_network_stats(message.body).unwrap();
        assert_eq!(rates[0].transmit_mbps, 1200);
        assert_eq!(rates[0].receive_mbps, 1100);
    }

    #[test]
    fn network_stats_rejects_missing_count_and_truncated_records() {
        assert!(parse_network_stats(&[]).is_err());
        assert!(parse_network_stats(&[1]).is_err());
    }
}
