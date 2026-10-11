//! Raw Ethernet transport implementations.

mod linux;
#[cfg(windows)]
mod windows;

pub use linux::LinuxTransport;
#[cfg(windows)]
pub use windows::WindowsTransport;

use crate::{Error, MacAddress};
use std::time::Duration;

pub trait Transport {
    fn exchange(
        &mut self,
        destination: MacAddress,
        ethertype: u16,
        payload: &[u8],
        timeout: Duration,
    ) -> Result<Vec<Vec<u8>>, Error>;
}
