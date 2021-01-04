use byteorder::{BigEndian, ReadBytesExt};
use std::fmt;
use std::io::{self, Cursor};

use crate::common::FromBytes;

/// Vector type with signed 16-bit coordinate values.
pub struct Vec3s {
    pub x: i16,
    pub y: i16,
    pub z: i16,
}

impl Vec3s {
    /// Size of `Vec3s` when serialized.
    pub const SIZE: usize = 6;

    pub fn from(x: i16, y: i16, z: i16) -> Self {
        Self {
            x,
            y,
            z,
        }
    }

    pub fn to_tuple(&self) -> (i16, i16, i16) {
        (self.x, self.y, self.z)
    }
}

impl FromBytes for Vec3s {
    fn from_bytes(bytes: &[u8]) -> io::Result<Self> {
        let mut cursor = Cursor::new(bytes);
        let x = cursor.read_i16::<BigEndian>()?;
        let y = cursor.read_i16::<BigEndian>()?;
        let z = cursor.read_i16::<BigEndian>()?;
        Ok(Vec3s::from(x, y, z))
    }
}

impl fmt::Display for Vec3s {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({0}, {1}, {2})", self.x, self.y, self.z)
    }
}
