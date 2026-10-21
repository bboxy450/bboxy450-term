//! Canonical ULID values without RNG or a runtime library. Creation belongs to
//! the app/mux; the model accepts the 128-bit value and distinguishes ID domains.

use std::{fmt, str::FromStr};

const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ulid(pub u128);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidUlid;
impl fmt::Display for InvalidUlid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("expected a canonical 26-character ULID")
    }
}
impl std::error::Error for InvalidUlid {}

impl Ulid {
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(u128::from_be_bytes(bytes))
    }
    pub const fn to_bytes(self) -> [u8; 16] {
        self.0.to_be_bytes()
    }
}
impl FromStr for Ulid {
    type Err = InvalidUlid;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.len() != 26 {
            return Err(InvalidUlid);
        }
        let mut value = 0u128;
        for byte in text.bytes() {
            let digit = ALPHABET
                .iter()
                .position(|c| *c == byte)
                .ok_or(InvalidUlid)?;
            value = value
                .checked_mul(32)
                .and_then(|v| v.checked_add(digit as u128))
                .ok_or(InvalidUlid)?;
        }
        Ok(Self(value))
    }
}
impl fmt::Display for Ulid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut bytes = [b'0'; 26];
        let mut value = self.0;
        for byte in bytes.iter_mut().rev() {
            *byte = ALPHABET[(value & 31) as usize];
            value >>= 5;
        }
        f.write_str(std::str::from_utf8(&bytes).expect("ASCII ULID alphabet"))
    }
}

macro_rules! ids {
    ($($name:ident),+ $(,)?) => {$ (
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub Ulid);
        impl $name { pub const fn from_u128(value: u128) -> Self { Self(Ulid(value)) } }
        impl fmt::Display for $name { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { self.0.fmt(f) } }
        impl FromStr for $name { type Err = InvalidUlid; fn from_str(s: &str) -> Result<Self, Self::Err> { Ok(Self(s.parse()?)) } }
    )+};
}
ids!(SeatId, SwarmId, TabId, ShadowId, TaskId, MessageId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RowId {
    Seat(SeatId),
    Shadow(ShadowId),
    Tab(TabId),
    Swarm(SwarmId),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_ulid_round_trip_and_overflow() {
        for value in [0, 1, u64::MAX as u128, u128::MAX] {
            let id = Ulid(value);
            assert_eq!(id.to_string().parse::<Ulid>().unwrap(), id);
            assert_eq!(Ulid::from_bytes(id.to_bytes()), id);
        }
        assert_eq!(Ulid(u128::MAX).to_string(), "7ZZZZZZZZZZZZZZZZZZZZZZZZZ");
        for invalid in [
            "8ZZZZZZZZZZZZZZZZZZZZZZZZZ",
            "0000000000000000000000000I",
            "01arz3ndektsv4rrffq69g5fav",
            "short",
        ] {
            assert!(invalid.parse::<SeatId>().is_err());
        }
        let known = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
        assert_eq!(known.parse::<SwarmId>().unwrap().to_string(), known);
    }
}
