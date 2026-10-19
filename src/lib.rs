//! HomePlug AV/AV2 discovery and diagnostics.

mod error;
mod homeplug;
mod mac;
mod transport;

pub use error::Error;
pub use homeplug::{
    APPLY_CONFIRM, BROADCOM_OUI, BroadcomNetwork, DiscoveredNetwork, DiscoveredStation,
    ETHERTYPE_HPAV, ETHERTYPE_MEDIAXTREAM, GET_PARAM_CONFIRM, ManagementMessage,
    PARAM_POWER_STANDBY, PARAM_POWER_STANDBY_AUX, PhyRate, PowerSavingState, SET_PARAM_ALT_CONFIRM,
    SET_PARAM_CONFIRM, StationCapabilities, encode_apply, encode_broadcom_network_info_request,
    encode_get_parameter, encode_set_parameter, parse_broadcom_network_info, parse_discover_list,
    parse_network_stats, parse_parameter_value, parse_station_capabilities, strip_vendor_prefix,
};
pub use mac::MacAddress;
#[cfg(windows)]
pub use transport::WindowsTransport;
pub use transport::{LinuxTransport, Transport};
