use crate::Error;
use serde::Serialize;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MacAddress(pub [u8; 6]);

impl MacAddress {
    pub const BROADCAST: Self = Self([0xff; 6]);

    pub fn parse(value: &str) -> Result<Self, Error> {
        let bytes: Result<Vec<_>, _> = value
            .split(':')
            .map(|b| u8::from_str_radix(b, 16))
            .collect();
        let bytes = bytes.map_err(|_| Error::InvalidMac(value.to_owned()))?;
        let bytes: [u8; 6] = bytes
            .try_into()
            .map_err(|_| Error::InvalidMac(value.to_owned()))?;
        Ok(Self(bytes))
    }
}

impl fmt::Display for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let b = self.0;
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            b[0], b[1], b[2], b[3], b[4], b[5]
        )
    }
}

impl Serialize for MacAddress {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
