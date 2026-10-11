//! HomePlug AV/AV2 discovery and diagnostics.

mod error;
mod homeplug;
mod mac;
mod transport;

pub use error::Error;
pub use homeplug::{
    BROADCOM_OUI, BroadcomNetwork, DiscoveredNetwork, DiscoveredStation, ETHERTYPE_HPAV,
    ETHERTYPE_MEDIAXTREAM, ManagementMessage, PhyRate, StationCapabilities,
    encode_broadcom_network_info_request, parse_broadcom_network_info, parse_discover_list,
    parse_network_stats, parse_station_capabilities,
};
pub use mac::MacAddress;
pub use transport::{LinuxTransport, Transport};
