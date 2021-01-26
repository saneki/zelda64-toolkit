use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::convert::TryInto;
use std::fmt;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::ops::Range;
use thiserror::Error;

use crate::util;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    IOError(#[from] io::Error),
    #[error("Invalid header magic")]
    InvalidHeader,
    #[error("Invalid mapping range")]
    InvalidRange(Mapping, Range<u32>),
}

/// Custom Result type.
pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Debug)]
/// Mapping type.
///
/// Currently only used in Error.
pub enum Mapping {
    Physical,
    Virtual,
}

impl fmt::Display for Mapping {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Physical => write!(f, "physical"),
            Self::Virtual => write!(f, "virtual"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Entry {
    values: [u32; 4],
}

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let virt = self.virt();
        let (phys, kind) = self.range();
        match kind {
            EntryType::Compressed | EntryType::Decompressed => {
                let phys = phys.unwrap();
                let indicator = match kind {
                    EntryType::Compressed => "C",
                    EntryType::Decompressed => "D",
                    _ => unreachable!(),
                };
                write!(f, "[{}]: Virtual({:08X}, {:08X}) | Physical({:08X}, {:08X})",
                    indicator, virt.start, virt.end, phys.start, phys.end)?;
                let diff = self.diff();
                match diff {
                    Ok(diff) => {
                        // If Compressed/Decompressed and not error, should always be a valid int.
                        let diff = diff.unwrap();
                        match diff {
                            0 => Ok(()),
                            _ => write!(f, " | Diff=0x{}", util::to_signed_hex(diff))
                        }
                    }
                    Err(err) => write!(f, " | {}", err)
                }
            }
            EntryType::DoesNotExist => {
                write!(f, "[N]: Virtual({:08X}, {:08X}) | Physical([DoesNotExist])",
                    virt.start, virt.end)
            }
            EntryType::Empty => write!(f, "[Empty]"),
        }
    }
}

impl AsRef<[u32; 4]> for Entry {
    fn as_ref(&self) -> &[u32; 4] {
        &self.values
    }
}

impl AsMut<[u32; 4]> for Entry {
    fn as_mut(&mut self) -> &mut [u32; 4] {
        &mut self.values
    }
}

impl Entry {
    const SIZE: usize = 0x10;

    /// Virtual start address.
    pub fn virt_start(&self) -> u32 {
        self.values[0]
    }

    /// Virtual end address.
    pub fn virt_end(&self) -> u32 {
        self.values[1]
    }

    /// Physical start address.
    pub fn phys_start(&self) -> u32 {
        self.values[2]
    }

    /// Physical end address.
    pub fn phys_end(&self) -> u32 {
        self.values[3]
    }

    /// Gets difference between uncompressed and compressed sizes.
    pub fn diff(&self) -> Result<Option<isize>> {
        let (virt, phys, _) = self.validate()?;
        match phys {
            Some(phys) => {
                let vlen: isize = virt.len().try_into().unwrap();
                let plen: isize = phys.len().try_into().unwrap();
                let diff = Some(vlen - plen);
                Ok(diff)
            }
            _ => Ok(None),
        }
    }

    pub fn from(virt_start: u32, virt_end: u32, phys_start: u32, phys_end: u32) -> Self {
        Self {
            values: [virt_start, virt_end, phys_start, phys_end],
        }
    }

    pub fn from_decompressed(virt: Range<u32>, phys_start: u32) -> Self {
        Self::from(virt.start, virt.end, phys_start, 0)
    }

    pub fn from_range(virt: Range<u32>, phys: Range<u32>) -> Self {
        Self::from(virt.start, virt.end, phys.start, phys.end)
    }

    /// Create expected initial `Entry`.
    pub fn initial() -> Self {
        Self::from(0, 0x1060, 0, 0)
    }

    /// Get the respective EntryType.
    pub fn kind(&self) -> EntryType {
        let phys = self.phys();
        if self.as_ref().into_iter().all(|&x| x == 0) {
            EntryType::Empty
        } else if phys.start == ::std::u32::MAX && phys.end == ::std::u32::MAX {
            EntryType::DoesNotExist
        } else if phys.end == 0 {
            EntryType::Decompressed
        } else {
            EntryType::Compressed
        }
    }

    pub fn read<T: Read>(reader: &mut T) -> io::Result<Self> {
        let virt_start = reader.read_u32::<BigEndian>()?;
        let virt_end = reader.read_u32::<BigEndian>()?;
        let phys_start = reader.read_u32::<BigEndian>()?;
        let phys_end = reader.read_u32::<BigEndian>()?;
        let entry = Self::from(virt_start, virt_end, phys_start, phys_end);
        Ok(entry)
    }

    /// Get physical start and end addresses as a `Range`.
    pub fn phys(&self) -> Range<u32> {
        self.phys_start()..self.phys_end()
    }

    /// Get virtual start and end addresses as a `Range`.
    pub fn virt(&self) -> Range<u32> {
        self.virt_start()..self.virt_end()
    }

    /// Get the "real" address `Range` of file data relative to ROM start.
    pub fn range(&self) -> (Option<Range<u32>>, EntryType) {
        let kind = self.kind();
        match kind {
            EntryType::Compressed => (Some(self.phys()), kind),
            EntryType::Decompressed => {
                // If decompressed, physical mapping end will be 0, thus use virtual mapping range length.
                let length = self.virt().len() as u32;
                let range = self.phys_start()..self.phys_start() + length;
                (Some(range), kind)
            }
            _ => (None, kind),
        }
    }

    /// Validate this table entry.
    pub fn validate(&self) -> Result<(Range<u32>, Option<Range<u32>>, EntryType)> {
        let virt = self.virt();
        let (phys, kind) = self.range();

        if virt.start > virt.end {
            Err(Error::InvalidRange(Mapping::Virtual, virt))
        } else {
            match phys {
                Some(phys) => {
                    if phys.start > phys.end {
                        Err(Error::InvalidRange(Mapping::Physical, phys))
                    } else {
                        Ok((virt, Some(phys), kind))
                    }
                }
                None => Ok((virt, None, kind)),
            }
        }
    }

    /// Write.
    pub fn write<T: Write>(&self, writer: &mut T) -> io::Result<usize> {
        writer.write_u32::<BigEndian>(self.virt_start())?;
        writer.write_u32::<BigEndian>(self.virt_end())?;
        writer.write_u32::<BigEndian>(self.phys_start())?;
        writer.write_u32::<BigEndian>(self.phys_end())?;
        Ok(Self::SIZE)
    }
}

pub enum EntryType {
    /// Entry file is compressed.
    Compressed,

    /// Entry file is decompressed.
    Decompressed,

    /// Entry file does not exist (physical addresses are both 0xFFFFFFFF).
    DoesNotExist,

    /// Entry file is empty (all fields are 0).
    Empty,
}

pub struct Table {
    /// `dmadata` entries.
    pub entries: Vec<Entry>,
}

impl fmt::Display for Table {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for entry in &self.entries {
            writeln!(f, "{}", entry)?;
        }
        Ok(())
    }
}

impl Table {
    pub fn from(entries: Vec<Entry>) -> Self {
        Self {
            entries,
        }
    }

    /// Find `Table` in ROM and return along with offset.
    pub fn find<T: Read + Seek>(mut stream: &mut T) -> Result<Option<(Table, usize)>> {
        let offset = Self::find_offset(stream)?;
        match offset {
            Some(offset) => {
                stream.seek(SeekFrom::Start(offset))?;
                let table = Self::read(&mut stream)?;
                let origin: usize = (offset as usize).try_into().unwrap();
                Ok(Some((table, origin)))
            }
            None => Ok(None),
        }
    }

    /// Read `Table` from reader at given offset. Assumes the reader is already positioned at this offset.
    pub fn read_at<T: Read>(mut reader: &mut T, begin: u32) -> Result<Table> {
        let mut current: usize = (begin as usize).try_into().unwrap();
        let mut entries = Vec::new();
        let mut end = None;
        loop {
            let entry = Entry::read(&mut reader)?;

            // Table should include an entry about itself, it should be uncompressed.
            if end == None && entry.virt_start() == begin {
                end = Some(entry.virt_end() as usize);
            }

            // Check if the end has been reached.
            match end {
                Some(end) => {
                    if current >= end {
                        break;
                    }
                }
                _ => (),
            }

            entries.push(entry);
            current += Entry::SIZE;
        }
        let table = Table::from(entries);
        Ok(table)
    }

    /// Read `Table` from stream.
    pub fn read<T: Read + Seek>(mut stream: &mut T) -> Result<Table> {
        let offset = stream.seek(SeekFrom::Current(0))?;
        let begin = (offset as u32).try_into().unwrap();
        Self::read_at(&mut stream, begin)
    }

    /// Find the offset of the DMA table, relative to start of stream.
    pub fn find_offset<T: Read + Seek>(stream: &mut T) -> Result<Option<u64>> {
        let initial = Entry::initial();
        stream.seek(SeekFrom::Start(0))?;
        loop {
            let entry = Entry::read(stream)?;
            if entry == initial {
                let delta: u64 = (Entry::SIZE as u64).try_into().unwrap();
                let result = stream.seek(SeekFrom::Current(0))? - delta;
                return Ok(Some(result))
            }
        }
    }

    /// Write `Table` entries to writer.
    pub fn write<T: Write>(&self, mut writer: &mut T) -> Result<usize> {
        let mut length = 0;
        for entry in &self.entries {
            length = length + entry.write(&mut writer)?;
        }
        Ok(length)
    }
}
