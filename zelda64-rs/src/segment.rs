use std::fmt;

/// Attaches a `SegAddr` to an instance of another type.
pub struct Relative<T> {
    pub address: SegAddr,
    pub value: T,
}

impl<T> Relative<T> {
    pub fn from(address: SegAddr, value: T) -> Self {
        Self {
            address,
            value,
        }
    }
}

/// Segmented address, with an 8-bit segment index and 24-bit offset.
pub struct SegAddr(u32);

impl SegAddr {
    pub fn from(segment: u8, offset: u32) -> Self {
        let value: u32 = ((segment as u32) << 24) | (offset & 0xFFFFFF);
        Self(value)
    }

    pub fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// Get offset value.
    pub fn offset(&self) -> u32 {
        self.0 & 0xFFFFFF
    }

    /// Get raw value.
    pub fn raw(&self) -> u32 {
        self.0
    }

    /// Get segment index.
    pub fn segment(&self) -> u8 {
        (self.0 >> 24) as u8
    }
}

impl fmt::Display for SegAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{0:02X}:{1:06X}", self.segment(), self.offset())
    }
}
