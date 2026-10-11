//! HomePlug MME framing and typed management message bodies.

mod broadcom;
mod capabilities;
mod discover;
mod network_stats;

pub use broadcom::{
    BroadcomNetwork, encode_broadcom_network_info_request, parse_broadcom_network_info,
};
pub use capabilities::{StationCapabilities, parse_station_capabilities};
pub use discover::{DiscoveredNetwork, DiscoveredStation, parse_discover_list};
pub use network_stats::{PhyRate, parse_network_stats};

pub const ETHERTYPE_HPAV: u16 = 0x88e1;
pub const ETHERTYPE_MEDIAXTREAM: u16 = 0x8912;
pub const BROADCOM_OUI: [u8; 3] = [0x00, 0x1f, 0x84];

use crate::Error;
use zerocopy::byteorder::little_endian::U16 as LeU16;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

#[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
struct ManagementHeader {
    version: u8,
    mmtype: LeU16,
    fragment_info: u8,
    fragment_sequence: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagementMessage<'a> {
    pub version: u8,
    pub mmtype: u16,
    pub fragment_info: u8,
    pub fragment_sequence: u8,
    pub body: &'a [u8],
}

impl<'a> ManagementMessage<'a> {
    pub fn parse(payload: &'a [u8]) -> Result<Self, Error> {
        let (header, body) = ManagementHeader::ref_from_prefix(payload)
            .map_err(|_| Error::Truncated("HomePlug AV management header"))?;
        if header.version != 1 && header.version != 2 {
            return Err(Error::Invalid("HomePlug AV version"));
        }
        Ok(Self {
            version: header.version,
            mmtype: header.mmtype.get(),
            fragment_info: header.fragment_info,
            fragment_sequence: header.fragment_sequence,
            body,
        })
    }

    pub fn encode(version: u8, mmtype: u16, body: &[u8]) -> Vec<u8> {
        let header = ManagementHeader {
            version,
            mmtype: LeU16::new(mmtype),
            fragment_info: 0,
            fragment_sequence: 0,
        };
        let mut bytes = Vec::with_capacity(header.as_bytes().len() + body.len());
        bytes.extend_from_slice(header.as_bytes());
        bytes.extend_from_slice(body);
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::ManagementMessage;

    #[test]
    fn standard_request_encoder_matches_wire_bytes() {
        assert_eq!(
            ManagementMessage::encode(1, 0x0014, &[]),
            [0x01, 0x14, 0, 0, 0]
        );
    }

    #[test]
    fn management_header_rejects_truncation_and_unknown_version() {
        assert!(ManagementMessage::parse(&[2, 1, 2, 3]).is_err());
        assert!(ManagementMessage::parse(&[3, 1, 2, 3, 4]).is_err());
    }
}
