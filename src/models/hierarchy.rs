use anyhow::Result;
use byteorder::{BigEndian, ReadBytesExt};
use std::fmt::{self, Write};
use std::io::{self, Cursor};

use crate::common::FromBytes;
use crate::primitive::Vec3s;
use crate::segment::{Relative, SegAddr};

pub type Hierarchy = HierarchyWith<Limb>;
pub type PlayerHierarchy = HierarchyWith<PlayerLimb>;

/// Hierarchy header structure.
pub struct Header {
    /// Segmented address to beginning of limb index.
    pub limbs: SegAddr,
    /// Count of limb indexes.
    pub count: u8,
    /// Count of display lists?
    pub display_lists: u8,
}

impl FromBytes for Header {
    fn from_bytes(bytes: &[u8]) -> io::Result<Self> {
        let mut cursor = Cursor::new(bytes);
        // Read each word.
        let address = cursor.read_u32::<BigEndian>()?;
        let word1 = cursor.read_u32::<BigEndian>()?;
        let word2 = cursor.read_u32::<BigEndian>()?;
        // Convert words to values.
        let limbs = SegAddr::from_raw(address);
        let count = (word1 >> 24) as u8;
        let display_lists = (word2 >> 24) as u8;
        let result = Self {
            limbs,
            count,
            display_lists,
        };
        Ok(result)
    }
}

impl fmt::Display for Header {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Limb Indexes:  {}\n", self.limbs)?;
        write!(formatter, "Limb Count:    0x{:02X}\n", self.count)?;
        write!(formatter, "Display Lists: 0x{:02X}\n", self.display_lists)
    }
}

/// Hierarchy limb for standard `Hierarchy`.
pub struct Limb {
    /// Translation relative to the limb's parent.
    pub translation: Vec3s,
    /// First child index in the limb list.
    pub child: u8,
    /// Next limb's index in the limb list.
    pub next: u8,
    /// Address to display list.
    pub display_list: SegAddr,
}

impl Limb {
    /// Size of `Limb` when serialized.
    pub const SIZE: usize = 0xC;
}

impl FromBytes for Limb {
    fn from_bytes(bytes: &[u8]) -> io::Result<Self> {
        let translation = Vec3s::from_bytes(bytes)?;
        let mut cursor = Cursor::new(&bytes[Vec3s::SIZE..]);
        let child = cursor.read_u8()?;
        let next = cursor.read_u8()?;
        let display_list = SegAddr::from_raw(cursor.read_u32::<BigEndian>()?);
        let limb = Limb {
            translation,
            child,
            next,
            display_list,
        };
        Ok(limb)
    }
}

impl fmt::Display for Limb {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Translation: {}\n", self.translation)?;
        write!(formatter, "Child Limb:  {:#02X}\n", self.child)?;
        write!(formatter, "Next Limb:   {:#02X}\n", self.next)?;
        write!(formatter, "DisplayList: {}\n", self.display_list)
    }
}

/// Hierarchy limb for `PlayerHierarchy`.
pub struct PlayerLimb {
    /// Base limb structure.
    pub base: Limb,
    /// Address to "far model" display list.
    pub far_model_display_list: SegAddr,
}

impl FromBytes for PlayerLimb {
    fn from_bytes(bytes: &[u8]) -> io::Result<Self> {
        let base = Limb::from_bytes(bytes)?;
        let mut cursor = Cursor::new(&bytes[Limb::SIZE..]);
        let address = cursor.read_u32::<BigEndian>()?;
        let far_model_display_list = SegAddr::from_raw(address);
        let player_limb = Self {
            base,
            far_model_display_list,
        };
        Ok(player_limb)
    }
}

impl fmt::Display for PlayerLimb {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.base)?;
        write!(formatter, "Far Model:   {}\n", self.far_model_display_list)
    }
}

pub struct HierarchyWith<T: fmt::Display + FromBytes> {
    pub header: Header,
    pub limbs: Vec<Relative<T>>,
}

impl<T: fmt::Display + FromBytes> HierarchyWith<T> {
    pub fn from(header: Header, limbs: Vec<Relative<T>>) -> Self {
        Self {
            header,
            limbs,
        }
    }

    /// Read from object data with `Header` at specified offset.
    ///
    /// TODO: Ensure base segment index matches `header.limbs.segment()`?
    pub fn read_from(bytes: &[u8], offset: u32, _base: SegAddr) -> io::Result<Self> {
        let header = Header::from_bytes(&bytes[(offset as usize)..])?;
        let indexes_offset = header.limbs.offset() as usize;
        let mut cursor = Cursor::new(&bytes[indexes_offset..]);
        let mut limbs = Vec::with_capacity(header.count as usize);
        for _ in 0..header.count {
            let index = SegAddr::from_raw(cursor.read_u32::<BigEndian>()?);
            let limb_offset = index.offset() as usize;
            let limb = T::from_bytes(&bytes[limb_offset..])?;
            let relative = Relative::from(index, limb);
            limbs.push(relative);
        }
        Ok(Self::from(header, limbs))
    }

    pub fn to_paragraph<W: io::Write>(&self, writer: &mut W) -> Result<()> {
        write!(writer, "Header:\n")?;
        write!(writer, "{}\n", self.header)?;

        // Print each limb info, padded.
        let mut buffer = String::new();
        for (idx, limb) in self.limbs.iter().enumerate() {
            write!(writer, "Limb [{0}]: {1}\n", idx, limb.address)?;
            // Write limb info to temp buffer for padding.
            write!(&mut buffer, "{}", limb.value)?;
            for line in buffer.lines() {
                write!(writer, " {}", line)?;
            }
            buffer.clear();
        }
        Ok(())
    }
}
