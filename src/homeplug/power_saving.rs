use super::{BROADCOM_OUI, ManagementMessage};
use crate::Error;
use serde::Serialize;

pub const PARAM_POWER_STANDBY: u16 = 0x0029;
pub const PARAM_POWER_STANDBY_AUX: u16 = 0x0074;
pub const GET_PARAM_CONFIRM: u16 = 0xa05d;
pub const SET_PARAM_CONFIRM: u16 = 0xa059;
pub const SET_PARAM_ALT_CONFIRM: u16 = 0xa069;
pub const APPLY_CONFIRM: u16 = 0xa021;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct PowerSavingState {
    pub enabled: bool,
    pub standby_timeout_seconds: u16,
}

impl PowerSavingState {
    pub fn from_bytes(value: &[u8]) -> Result<Self, Error> {
        let raw = u16::from_le_bytes(
            value
                .try_into()
                .map_err(|_| Error::Invalid("standby parameter length"))?,
        );
        Ok(Self {
            enabled: (raw & 0x8000) != 0,
            standby_timeout_seconds: raw & 0x7fff,
        })
    }

    pub fn with_enabled(self, enabled: bool) -> [u8; 2] {
        (self.standby_timeout_seconds | if enabled { 0x8000 } else { 0 }).to_le_bytes()
    }
}

fn payload_prefix(sequence: u8) -> [u8; 4] {
    [BROADCOM_OUI[0], BROADCOM_OUI[1], BROADCOM_OUI[2], sequence]
}

pub fn encode_get_parameter(sequence: u8, id: u16) -> Vec<u8> {
    let mut body = payload_prefix(sequence).to_vec();
    body.extend_from_slice(&id.to_le_bytes());
    ManagementMessage::encode(2, 0xa05c, &body)
}

pub fn encode_set_parameter(sequence: u8, id: u16, value: &[u8]) -> Vec<u8> {
    let mut body = payload_prefix(sequence).to_vec();
    body.extend_from_slice(&id.to_le_bytes());
    body.push(value.len() as u8);
    body.extend_from_slice(&1_u16.to_le_bytes());
    body.extend_from_slice(value);
    ManagementMessage::encode(2, 0xa058, &body)
}

pub fn encode_apply(sequence: u8) -> Vec<u8> {
    ManagementMessage::encode(2, 0xa020, &payload_prefix(sequence))
}

pub fn strip_vendor_prefix(body: &[u8], sequence: u8) -> Result<&[u8], Error> {
    let (prefix, payload) = body
        .split_at_checked(4)
        .ok_or(Error::Truncated("Mediaxtream OUI and sequence"))?;
    if prefix != payload_prefix(sequence) {
        return Err(Error::Invalid("Mediaxtream OUI or sequence"));
    }
    Ok(payload)
}

pub fn parse_parameter_value(body: &[u8], sequence: u8) -> Result<&[u8], Error> {
    let payload = strip_vendor_prefix(body, sequence)?;
    let (&width, rest) = payload
        .split_first()
        .ok_or(Error::Truncated("parameter element width"))?;
    let (count, value) = rest
        .split_at_checked(2)
        .ok_or(Error::Truncated("parameter element count"))?;
    if !matches!(width, 1 | 2 | 4) {
        return Err(Error::Invalid("parameter element width"));
    }
    let count = u16::from_le_bytes([count[0], count[1]]) as usize;
    let len = usize::from(width) * count;
    if count == 0 || len > 4096 {
        return Err(Error::Invalid("parameter element count"));
    }
    value.get(..len).ok_or(Error::Truncated("parameter value"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mediaxtream_parameter_wire_frames() {
        assert_eq!(
            encode_get_parameter(7, PARAM_POWER_STANDBY),
            [2, 0x5c, 0xa0, 0, 0, 0, 0x1f, 0x84, 7, 0x29, 0]
        );
        assert_eq!(
            encode_set_parameter(8, PARAM_POWER_STANDBY, &[0x2c, 0x01]),
            [
                2, 0x58, 0xa0, 0, 0, 0, 0x1f, 0x84, 8, 0x29, 0, 2, 1, 0, 0x2c, 1
            ]
        );
        assert_eq!(encode_apply(9), [2, 0x20, 0xa0, 0, 0, 0, 0x1f, 0x84, 9]);
    }

    #[test]
    fn reads_and_preserves_timeout() {
        let payload = [0, 0x1f, 0x84, 6, 2, 1, 0, 0x2c, 0x81];
        let state =
            PowerSavingState::from_bytes(parse_parameter_value(&payload, 6).unwrap()).unwrap();
        assert!(state.enabled);
        assert_eq!(state.standby_timeout_seconds, 300);
        assert_eq!(state.with_enabled(false), [0x2c, 0x01]);
        assert_eq!(state.with_enabled(true), [0x2c, 0x81]);
    }

    #[test]
    fn rejects_wrong_vendor_sequence_and_malformed_parameter() {
        assert!(parse_parameter_value(&[0, 0x1f, 0x84, 6, 2, 1, 0, 0x2c, 0x81], 5).is_err());
        assert!(parse_parameter_value(&[0, 0x1f, 0x84, 6, 2, 3, 0, 0x2c, 0x81], 6).is_err());
        assert!(parse_parameter_value(&[0, 0x1f, 0x84, 6, 2, 1, 0, 0x2c], 6).is_err());
        assert!(parse_parameter_value(&[], 6).is_err());
        assert!(PowerSavingState::from_bytes(&[0]).is_err());
    }
}
