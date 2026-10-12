use super::Transport;
use crate::{Error, MacAddress};
use etherparse::{EtherType, Ethernet2Header, Ethernet2Slice};
use pcap::{Capture, Device};
use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{ERROR_BUFFER_OVERFLOW, ERROR_SUCCESS};
use windows_sys::Win32::NetworkManagement::IpHelper::{
    GAA_FLAG_INCLUDE_PREFIX, GetAdaptersAddresses, IP_ADAPTER_ADDRESSES_LH,
};
use windows_sys::Win32::Networking::WinSock::AF_UNSPEC;

pub struct WindowsTransport {
    device: Device,
    pub source: MacAddress,
}

#[derive(Clone)]
struct Adapter {
    guid: String,
    friendly_name: String,
    description: String,
    mac: Option<MacAddress>,
}

impl WindowsTransport {
    pub fn new(interface: &str) -> Result<Self, Error> {
        let devices = Device::list()
            .map_err(|e| Error::Transport(format!("Npcap device enumeration failed: {e}")))?;
        let adapters = enumerate_adapters()?;
        let normalized_interface = normalize_guid(interface);
        let mut matches = Vec::new();

        for device in devices {
            let Some(device_guid) = guid_from_pcap_name(&device.name) else {
                continue;
            };
            let Some(adapter) = adapters
                .iter()
                .find(|a| a.guid.eq_ignore_ascii_case(&device_guid))
            else {
                continue;
            };
            let aliases = [
                device.name.as_str(),
                device.desc.as_deref().unwrap_or_default(),
                adapter.friendly_name.as_str(),
                adapter.description.as_str(),
                adapter.guid.as_str(),
            ];
            if aliases
                .iter()
                .any(|name| name.eq_ignore_ascii_case(interface))
                || adapter.guid == normalized_interface
            {
                matches.push((device, adapter.mac));
            }
        }

        let (device, mac) = match matches.len() {
            0 => {
                return Err(Error::Transport(format!(
                    "no Npcap interface matches '{interface}'; select its Npcap name, Windows friendly name, interface description, or adapter GUID"
                )));
            }
            1 => matches.pop().ok_or_else(|| {
                Error::Transport("matched Npcap interface disappeared during selection".to_owned())
            })?,
            count => {
                return Err(Error::Transport(format!(
                    "interface '{interface}' is ambiguous ({count} Npcap adapters match); select the Npcap device name or adapter GUID"
                )));
            }
        };
        let source = mac.ok_or_else(|| {
            Error::Transport(format!(
                "selected interface '{}' does not expose a six-byte physical MAC address",
                device.name
            ))
        })?;

        Ok(Self { device, source })
    }
}

impl Transport for WindowsTransport {
    fn exchange(
        &mut self,
        destination: MacAddress,
        ethertype: u16,
        payload: &[u8],
        timeout: Duration,
    ) -> Result<Vec<Vec<u8>>, Error> {
        let header = Ethernet2Header {
            destination: destination.0,
            source: self.source.0,
            ether_type: EtherType(ethertype),
        }
        .to_bytes();
        let mut frame = Vec::with_capacity(60.max(header.len() + payload.len()));
        frame.extend_from_slice(&header);
        frame.extend_from_slice(payload);
        frame.resize(frame.len().max(60), 0);

        let mut capture = Capture::from_device(self.device.clone())
            .map_err(pcap_error)?
            .snaplen(65535)
            .promisc(false)
            .timeout(30)
            .immediate_mode(true)
            .open()
            .map_err(pcap_error)?;
        capture.sendpacket(frame).map_err(pcap_error)?;

        let deadline = Instant::now() + timeout;
        let mut responses = Vec::new();
        loop {
            if Instant::now() >= deadline {
                break;
            }
            match capture.next_packet() {
                Ok(packet) => {
                    let bytes = packet.data;
                    if Ethernet2Slice::from_slice_without_fcs(bytes)
                        .is_ok_and(|parsed| parsed.ether_type().0 == ethertype)
                    {
                        responses.push(bytes.to_vec());
                    }
                }
                Err(pcap::Error::TimeoutExpired) => {}
                Err(error) => return Err(pcap_error(error)),
            }
        }
        Ok(responses)
    }
}

fn pcap_error(error: pcap::Error) -> Error {
    Error::Transport(format!("Npcap error: {error}"))
}

fn guid_from_pcap_name(name: &str) -> Option<String> {
    const PREFIX: &str = r"\Device\NPF_";
    let prefix = name.get(..PREFIX.len())?;
    if !prefix.eq_ignore_ascii_case(PREFIX) {
        return None;
    }
    Some(normalize_guid(&name[PREFIX.len()..]))
}

fn normalize_guid(value: &str) -> String {
    value
        .trim()
        .trim_start_matches('{')
        .trim_end_matches('}')
        .to_ascii_lowercase()
}

fn enumerate_adapters() -> Result<Vec<Adapter>, Error> {
    let mut bytes = 15_000usize;
    loop {
        let count = bytes.div_ceil(std::mem::size_of::<IP_ADAPTER_ADDRESSES_LH>());
        let mut storage: Vec<MaybeUninit<IP_ADAPTER_ADDRESSES_LH>> = Vec::with_capacity(count);
        // SAFETY: MaybeUninit permits uninitialized elements; set_len establishes the aligned
        // writable backing region expected by GetAdaptersAddresses.
        unsafe { storage.set_len(count) };
        let mut required = (count * std::mem::size_of::<IP_ADAPTER_ADDRESSES_LH>()) as u32;
        // SAFETY: storage is suitably aligned and writable, and required is a valid size pointer.
        let status = unsafe {
            GetAdaptersAddresses(
                AF_UNSPEC as u32,
                GAA_FLAG_INCLUDE_PREFIX,
                std::ptr::null(),
                storage.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>(),
                &mut required,
            )
        };
        if status == ERROR_BUFFER_OVERFLOW {
            bytes = required as usize;
            continue;
        }
        if status != ERROR_SUCCESS {
            return Err(Error::Transport(format!(
                "GetAdaptersAddresses failed with Windows error {status}"
            )));
        }

        let mut adapters = Vec::new();
        // SAFETY: on ERROR_SUCCESS, Windows returns a linked list contained in storage. Each
        // non-null node and its documented strings remain valid until storage is dropped.
        let mut current = storage.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
        while !current.is_null() {
            // SAFETY: current is a node returned in the successful API buffer.
            let raw = unsafe { &*current };
            let adapter_name = if raw.AdapterName.is_null() {
                String::new()
            } else {
                // SAFETY: AdapterName is a NUL-terminated ANSI string owned by the API buffer.
                unsafe { CStr::from_ptr(raw.AdapterName.cast()) }
                    .to_string_lossy()
                    .into_owned()
            };
            let mac = if raw.PhysicalAddressLength == 6 {
                let mut bytes = [0u8; 6];
                bytes.copy_from_slice(&raw.PhysicalAddress[..6]);
                Some(MacAddress(bytes))
            } else {
                None
            };
            adapters.push(Adapter {
                guid: normalize_guid(&adapter_name),
                friendly_name: wide_string(raw.FriendlyName),
                description: wide_string(raw.Description),
                mac,
            });
            current = raw.Next;
        }
        return Ok(adapters);
    }
}

fn wide_string(value: *const u16) -> String {
    if value.is_null() {
        return String::new();
    }
    // SAFETY: GetAdaptersAddresses returns NUL-terminated UTF-16 strings in its result buffer.
    let mut length = 0;
    unsafe {
        while *value.add(length) != 0 {
            length += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(value, length))
    }
}

#[cfg(test)]
mod tests {
    use super::{guid_from_pcap_name, normalize_guid};

    #[test]
    fn parses_npcap_and_adapter_guid_forms() {
        assert_eq!(
            guid_from_pcap_name(r"\Device\NPF_{01234567-89AB-CDEF-0123-456789ABCDEF}"),
            Some("01234567-89ab-cdef-0123-456789abcdef".to_owned())
        );
        assert_eq!(
            normalize_guid("{01234567-89AB-CDEF-0123-456789ABCDEF}"),
            "01234567-89ab-cdef-0123-456789abcdef"
        );
        assert_eq!(
            guid_from_pcap_name(r"\device\npf_{01234567-89AB-CDEF-0123-456789ABCDEF}"),
            Some("01234567-89ab-cdef-0123-456789abcdef".to_owned())
        );
        assert_eq!(guid_from_pcap_name("not-an-npcap-name"), None);
    }
}
