use std::fmt;

pub fn swap_bytes<T: Swap>(buf: &mut [u8]) {
    T::swap(buf)
}

pub trait Swap {
    fn swap(buf: &mut [u8]);
}

pub enum BigEndian {}
pub type BE = BigEndian;

impl Swap for BigEndian {
    fn swap(_: &mut [u8]) {
        // Empty
    }
}

pub enum LittleEndian {}
pub type LE = LittleEndian;

impl Swap for LittleEndian {
    fn swap(buf: &mut [u8]) {
        assert_eq!(buf.len() % 4, 0, "LittleEndian byte swapping requires a multiple of 4");
        let swaps = buf.len() / 4;
        for i in 0..swaps {
            let idx = i*4;
            let (temp2, temp1) = (buf[idx], buf[idx+1]);
            buf[idx] = buf[idx+3];
            buf[idx+1] = buf[idx+2];
            buf[idx+2] = temp1;
            buf[idx+3] = temp2;
        }
    }
}

pub enum Mixed {}
pub type MX = Mixed;

impl Swap for Mixed {
    fn swap(buf: &mut [u8]) {
        assert_eq!(buf.len() % 2, 0, "Mixed byte swapping requires a multiple of 2");
        let swaps = buf.len() / 2;
        for i in 0..swaps {
            let idx = i*2;
            buf.swap(idx, idx+1);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
/// Convenience wrapper enum around the separate Swap endianness enums.
pub enum Endianness {
    Big,
    Little,
    Mixed,
}

impl fmt::Display for Endianness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Big => write!(f, "Big Endian"),
            Self::Little => write!(f, "Little Endian"),
            Self::Mixed => write!(f, "Mixed"),
        }
    }
}

/// Wrapper implementation around the separate Swap endianness enums.
impl Endianness {
    pub fn swap(&self, buf: &mut [u8]) {
        match self {
            Self::Big => swap_bytes::<BigEndian>(buf),
            Self::Little => swap_bytes::<LittleEndian>(buf),
            Self::Mixed => swap_bytes::<Mixed>(buf),
        }
    }
}
