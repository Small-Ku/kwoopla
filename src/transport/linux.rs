use super::Transport;
use crate::{Error, MacAddress};
use std::time::Duration;

#[cfg(target_os = "linux")]
use etherparse::{EtherType, Ethernet2Header, Ethernet2Slice};

#[cfg(target_os = "linux")]
pub struct LinuxTransport {
    socket: std::os::fd::RawFd,
    ifindex: libc::c_int,
    pub source: MacAddress,
}

#[cfg(target_os = "linux")]
impl LinuxTransport {
    pub fn new(interface: &str) -> Result<Self, Error> {
        use std::{ffi::CString, os::fd::RawFd};
        let name = CString::new(interface).map_err(|_| Error::Invalid("interface name"))?;
        // SAFETY: if_nametoindex reads the NUL-terminated interface name.
        let ifindex = unsafe { libc::if_nametoindex(name.as_ptr()) } as libc::c_int;
        if ifindex == 0 {
            return Err(Error::Io(std::io::Error::last_os_error()));
        }
        let path = format!("/sys/class/net/{interface}/address");
        let source = MacAddress::parse(&std::fs::read_to_string(path)?.trim())?;
        // SAFETY: socket arguments follow Linux AF_PACKET raw socket API.
        let socket: RawFd = unsafe {
            libc::socket(
                libc::AF_PACKET,
                libc::SOCK_RAW,
                (libc::ETH_P_ALL as u16).to_be() as i32,
            )
        };
        if socket < 0 {
            return Err(Error::Io(std::io::Error::last_os_error()));
        }
        let addr = libc::sockaddr_ll {
            sll_family: libc::AF_PACKET as u16,
            sll_protocol: (libc::ETH_P_ALL as u16).to_be(),
            sll_ifindex: ifindex,
            sll_hatype: 0,
            sll_pkttype: 0,
            sll_halen: 0,
            sll_addr: [0; 8],
        };
        // SAFETY: addr is initialized and has the correct sockaddr_ll size.
        if unsafe {
            libc::bind(
                socket,
                &addr as *const _ as *const libc::sockaddr,
                std::mem::size_of_val(&addr) as libc::socklen_t,
            )
        } < 0
        {
            let e = std::io::Error::last_os_error();
            // SAFETY: socket was returned by socket and is owned here.
            unsafe { libc::close(socket) };
            return Err(Error::Io(e));
        }
        Ok(Self {
            socket,
            ifindex,
            source,
        })
    }
}

#[cfg(not(target_os = "linux"))]
impl LinuxTransport {
    pub fn new(_: &str) -> Result<Self, Error> {
        Err(Error::UnsupportedPlatform)
    }
}

#[cfg(target_os = "linux")]
impl Transport for LinuxTransport {
    fn exchange(
        &mut self,
        destination: MacAddress,
        ethertype: u16,
        payload: &[u8],
        timeout: Duration,
    ) -> Result<Vec<Vec<u8>>, Error> {
        use std::time::Instant;
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
        let mut addr = libc::sockaddr_ll {
            sll_family: libc::AF_PACKET as u16,
            sll_protocol: ethertype.to_be(),
            sll_ifindex: self.ifindex,
            sll_hatype: 0,
            sll_pkttype: 0,
            sll_halen: 6,
            sll_addr: [0; 8],
        };
        addr.sll_addr[..6].copy_from_slice(&destination.0);
        // SAFETY: frame and sockaddr are valid for the duration of sendto.
        if unsafe {
            libc::sendto(
                self.socket,
                frame.as_ptr() as *const _,
                frame.len(),
                0,
                &addr as *const _ as *const libc::sockaddr,
                std::mem::size_of_val(&addr) as libc::socklen_t,
            )
        } < 0
        {
            return Err(Error::Io(std::io::Error::last_os_error()));
        }
        let deadline = Instant::now() + timeout;
        let mut responses = Vec::new();
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            let timeval = libc::timeval {
                tv_sec: remaining.as_secs() as _,
                tv_usec: remaining.subsec_micros() as _,
            };
            // SAFETY: timeval points to a valid timeval with expected length.
            unsafe {
                libc::setsockopt(
                    self.socket,
                    libc::SOL_SOCKET,
                    libc::SO_RCVTIMEO,
                    &timeval as *const _ as *const _,
                    std::mem::size_of_val(&timeval) as libc::socklen_t,
                );
            }
            let mut buffer = [0u8; 2048];
            // SAFETY: buffer is writable and has the declared capacity.
            let n =
                unsafe { libc::recv(self.socket, buffer.as_mut_ptr() as *mut _, buffer.len(), 0) };
            if n < 0 {
                let e = std::io::Error::last_os_error();
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut
                {
                    break;
                }
                return Err(Error::Io(e));
            }
            let bytes = &buffer[..n as usize];
            if Ethernet2Slice::from_slice_without_fcs(bytes)
                .is_ok_and(|frame| frame.ether_type().0 == ethertype)
            {
                responses.push(bytes.to_vec());
            }
        }
        Ok(responses)
    }
}

#[cfg(not(target_os = "linux"))]
impl Transport for LinuxTransport {
    fn exchange(
        &mut self,
        _: MacAddress,
        _: u16,
        _: &[u8],
        _: Duration,
    ) -> Result<Vec<Vec<u8>>, Error> {
        Err(Error::UnsupportedPlatform)
    }
}

#[cfg(target_os = "linux")]
impl Drop for LinuxTransport {
    fn drop(&mut self) {
        // SAFETY: socket is owned by this instance and closed exactly once.
        unsafe { libc::close(self.socket) };
    }
}

#[cfg(not(target_os = "linux"))]
pub struct LinuxTransport;
