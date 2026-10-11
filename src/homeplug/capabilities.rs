use crate::{Error, MacAddress};
use serde::Serialize;
use zerocopy::{FromBytes, Immutable, KnownLayout, Unaligned};

#[derive(FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
struct StationCapabilitiesWire {
    av_version: u8,
    mac: [u8; 6],
    oui: [u8; 3],
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StationCapabilities {
    pub av_version: u8,
    pub mac: MacAddress,
    pub oui: [u8; 3],
}

pub fn parse_station_capabilities(body: &[u8]) -> Result<StationCapabilities, Error> {
    let (record, _) = StationCapabilitiesWire::ref_from_prefix(body)
        .map_err(|_| Error::Truncated("station capabilities"))?;
    Ok(StationCapabilities {
        av_version: record.av_version,
        mac: MacAddress(record.mac),
        oui: record.oui,
    })
}

#[cfg(test)]
mod tests {
    use super::parse_station_capabilities;

    #[test]
    fn station_capabilities_reject_truncated_body() {
        assert!(parse_station_capabilities(&[0; 9]).is_err());
    }
}
