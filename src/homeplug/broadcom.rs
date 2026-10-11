use super::{BROADCOM_OUI, ManagementMessage};
use crate::{Error, MacAddress};
use serde::Serialize;
use zerocopy::{FromBytes, Immutable, KnownLayout, Unaligned};

#[derive(FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
struct BroadcomResponsePrefix {
    _opaque: [u8; 4],
    count: u8,
}

#[derive(FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
struct BroadcomNetworkWire {
    network_id: [u8; 7],
    short_network_id: u8,
    terminal_equipment_id: u8,
    role: u8,
    coordinator_mac: [u8; 6],
    access_network: u8,
    coordinating_neighbor_network_count: u8,
    powerline_status: u8,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BroadcomNetwork {
    pub network_id: String,
    pub short_network_id: u8,
    pub terminal_equipment_id: u8,
    pub role: u8,
    pub coordinator_mac: MacAddress,
    pub access_network: bool,
    pub coordinating_neighbor_network_count: u8,
    pub powerline_status: u8,
}

pub fn encode_broadcom_network_info_request(any_network: bool) -> Vec<u8> {
    let mut body = BROADCOM_OUI.to_vec();
    body.extend_from_slice(&[0x01, 0x00, u8::from(any_network)]);
    ManagementMessage::encode(2, 0xa028, &body)
}

pub fn parse_broadcom_network_info(body: &[u8]) -> Result<Vec<BroadcomNetwork>, Error> {
    // The response prefix remains opaque; only its trailing record count is interpreted.
    let (prefix, body) = BroadcomResponsePrefix::ref_from_prefix(body)
        .map_err(|_| Error::Truncated("Broadcom network-info response prefix/count"))?;
    let (records, _) =
        <[BroadcomNetworkWire]>::ref_from_prefix_with_elems(body, prefix.count as usize)
            .map_err(|_| Error::Truncated("Broadcom network-info records"))?;
    Ok(records
        .iter()
        .map(|record| BroadcomNetwork {
            network_id: record
                .network_id
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
            short_network_id: record.short_network_id,
            terminal_equipment_id: record.terminal_equipment_id,
            role: record.role,
            coordinator_mac: MacAddress(record.coordinator_mac),
            access_network: record.access_network != 0,
            coordinating_neighbor_network_count: record.coordinating_neighbor_network_count,
            powerline_status: record.powerline_status,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::{encode_broadcom_network_info_request, parse_broadcom_network_info};
    use crate::MacAddress;

    #[test]
    fn broadcom_info_encoder_and_response_fixture() {
        assert_eq!(
            encode_broadcom_network_info_request(true),
            [2, 0x28, 0xa0, 0, 0, 0, 0x1f, 0x84, 1, 0, 1]
        );
        // Synthetic 19-byte record layout matched against a live BCM60500 response. All values
        // are placeholders; the fixture carries no device MAC, network ID, or captured bytes.
        let response = [
            0, 0, 0, 0, // opaque four-byte response prefix
            1, // one record
            1, 2, 3, 4, 5, 6, 7, // network ID
            8, // short network ID
            9, // terminal equipment ID
            3, // role: CCO in the hpavcli reference enum
            0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, // coordinator MAC placeholder
            0, 0, 0, // access, coordinating neighbor network count, status
        ];
        let networks = parse_broadcom_network_info(&response).unwrap();
        assert_eq!(networks[0].network_id, "01020304050607");
        assert_eq!(networks[0].role, 3);
        assert_eq!(
            networks[0].coordinator_mac,
            MacAddress([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff])
        );
        assert_eq!(networks[0].coordinating_neighbor_network_count, 0);
        assert_eq!(networks[0].powerline_status, 0);
        let json = serde_json::to_value(&networks[0]).unwrap();
        assert_eq!(json["coordinating_neighbor_network_count"], 0);
        assert!(json.get("station_count").is_none());

        // The live target returned both STA (2) and CCO (3) role codes.
        let mut sta_role = response;
        sta_role[14] = 2;
        assert_eq!(parse_broadcom_network_info(&sta_role).unwrap()[0].role, 2);

        // The count is the middle byte of the three-byte tail (access, numnets, status).
        let mut with_neighbors = response;
        with_neighbors[22] = 2;
        assert_eq!(
            parse_broadcom_network_info(&with_neighbors).unwrap()[0]
                .coordinating_neighbor_network_count,
            2
        );
        assert!(parse_broadcom_network_info(&response[..response.len() - 1]).is_err());
        assert!(parse_broadcom_network_info(&[]).is_err());
    }
}
