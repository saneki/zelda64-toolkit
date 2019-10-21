use byteorder::{BigEndian, ReadBytesExt};
use failure::Fail;
use n64rom::rom;
use std::convert::TryInto;
use std::fmt;
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::ops::Range;
use std::str::{self, Utf8Error};

use crate::util;

/// Table entry size.
const ENTRY_SIZE: usize = 0x10;

/// Table header size.
const HEADER_SIZE: usize = 0x30;

/// Table structure alignment.
const TABLE_ALIGN: usize = 0x10;

/// Table header magic string.
static TABLE_MAGIC: &'static [u8] = b"zelda@srd";

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "{}", _0)]
    IOError(#[cause] io::Error),

    #[fail(display = "Invalid header magic")]
    InvalidHeader,

    // #[fail(display = "{} start (0x{:08X}) cannot be greater than end (0x{:08X})", _0, _1.start, _1.end)]
    #[fail(display = "Invalid mapping range")]
    InvalidRange(Mapping, (Range<u32>)),

    #[fail(display = "Unknown table version")]
    UnknownVersion(Version),
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IOError(e)
    }
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

#[derive(Clone)]
pub struct Entry {
    virtual_start: u32,
    virtual_end: u32,
    physical_start: u32,
    physical_end: u32,
}

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let virt = self.virt();
        let (phys, kind) = self.real_phys();
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

impl Entry {
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

    /// Get the respective EntryType.
    pub fn kind(&self) -> EntryType {
        let phys = self.phys();

        if self.to_vec().iter().all(|&x| x == 0) {
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
        let virtual_start = reader.read_u32::<BigEndian>()?;
        let virtual_end = reader.read_u32::<BigEndian>()?;
        let physical_start = reader.read_u32::<BigEndian>()?;
        let physical_end = reader.read_u32::<BigEndian>()?;

        let entry = Self {
            virtual_start,
            virtual_end,
            physical_start,
            physical_end,
        };

        Ok(entry)
    }

    /// Get physical start and end addresses.
    pub fn phys(&self) -> Range<u32> {
        (self.physical_start..self.physical_end)
    }

    /// Get virtual start and end addresses.
    pub fn virt(&self) -> Range<u32> {
        (self.virtual_start..self.virtual_end)
    }

    /// Get the "real" physical address range.
    pub fn real_phys(&self) -> (Option<Range<u32>>, EntryType) {
        let kind = self.kind();
        match kind {
            EntryType::Compressed => (Some(self.phys()), kind),
            EntryType::Decompressed => {
                // If decompressed, physical mapping end will be 0
                // Thus use virtual mapping range length
                let length = self.virt().len() as u32;
                let range = self.physical_start..self.physical_start + length;
                (Some(range), kind)
            }
            _ => (None, kind),
        }
    }

    pub fn to_vec(&self) -> Vec<u32> {
        vec![
            self.virtual_start,
            self.virtual_end,
            self.physical_start,
            self.physical_end
        ]
    }

    /// Validate this table entry.
    pub fn validate(&self) -> Result<(Range<u32>, Option<Range<u32>>, EntryType)> {
        let virt = self.virt();
        let (phys, kind) = self.real_phys();

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

#[derive(Debug)]
pub enum Version {
    /// Found in:
    ///   Legend of Zelda, The - Ocarina of Time - Master Quest (U)
    Srd022J,
    /// Found in most other roms.
    Srd44,
    /// Unknown version identifier.
    Unknown(Vec<u8>),
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get_tag())
    }
}

impl Version {
    /// Determine from the version bytes found in a rom file.
    pub fn from(bytes: &[u8]) -> Self {
        match bytes {
            b"zelda@srd022j" => Self::Srd022J,
            b"zelda@srd44" => Self::Srd44,
            _ => Self::Unknown(bytes.to_vec()),
        }
    }

    pub fn get_tag(&self) -> &'static str {
        match self {
            Self::Srd022J => "srd022j",
            Self::Srd44 => "srd44",
            Self::Unknown(_) => "Unknown",
        }
    }

    /// Read the build date. Assumes the reader is immediately after the first null byte following
    /// the table magic string (such as "zelda@srd44").
    crate fn read_build_date<T: Read + Seek>(&self, mut stream: &mut T) -> Result<Vec<u8>> {
        let align: u64 = TABLE_ALIGN.try_into().unwrap();

        let result = match self {
            // For Srd022J, the build string will be aligned
            // Size of field: 0x20
            Version::Srd022J => {
                util::align_forward(&mut stream, align)?;
                let mut build_date = [0; 0x20];
                stream.read_exact(&mut build_date)?;
                build_date.to_vec().into_iter().take_while(|&x| x != 0).collect()
            }
            // For Srd44, the build string is immediately after.
            // Size of field: align(current) + 0x20
            Version::Srd44 => {
                let extra = util::get_align(&mut stream, align)?;
                let extra: usize = extra.try_into().unwrap();
                let mut build_date = vec![0; 0x20 + extra];
                stream.read_exact(&mut build_date)?;
                build_date.to_vec().into_iter().take_while(|&x| x != 0).collect()
            }
            _ => Vec::new()
        };

        Ok(result)
    }
}

/// Table header.
pub struct Header {
    version: Version,
    build_date: Vec<u8>,
}

impl fmt::Display for Header {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let date = self.build_date().unwrap_or("<Utf8Error>");
        writeln!(f, "Version: {}", &self.version)?;
        writeln!(f, "Build Date: {}", date)
    }
}

impl Header {
    /// Get the build date as a string.
    pub fn build_date(&self) -> ::std::result::Result<&str, Utf8Error> {
        str::from_utf8(&self.build_date)
    }

    /// Parse header from at least 0x30 bytes.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(&bytes);
        let (result, magic) = util::read_until(&mut cursor, 0, TABLE_ALIGN)?;
        if result {
            let version = Version::from(&magic);
            match version {
                Version::Unknown(_) => Err(Error::UnknownVersion(version)),
                _ => {
                    let raw_date = version.read_build_date(&mut cursor)?;
                    let build_date = raw_date.into_iter().take_while(|&x| x != 0).collect();
                    let header = Self {
                        version,
                        build_date,
                    };
                    Ok(header)
                }
            }
        } else {
            Err(Error::InvalidHeader)
        }
    }
}

pub struct Table {
    header: Header,
    entries: Vec<Entry>,
}

impl fmt::Display for Table {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", &self.header)?;
        for entry in &self.entries {
            writeln!(f, "{}", entry)?;
        }
        Ok(())
    }
}

impl Table {
    pub fn find<T: Read + Seek>(mut stream: &mut T) -> Result<Option<(Table, usize)>> {
        let results = Self::find_header(&mut stream)?;
        match results {
            Some((header, offset)) => {
                let mut entries = Vec::new();
                let mut current = offset + HEADER_SIZE;

                // We are working in a separate buffer from the rom header/ipl3.
                // The offsets we are reading, however, assume their presence.
                let begin: u32 = (rom::HEAD_SIZE + current).try_into().unwrap();
                let mut end = None;

                loop {
                    let entry = Entry::read(&mut stream)?;

                    // This table should include an entry about itself. It should be uncompressed.
                    if entry.virtual_start == begin {
                        end = Some((entry.virtual_end as usize) - rom::HEAD_SIZE);
                    }

                    // Check if we reached the end yet
                    match end {
                        Some(end) => {
                            if current >= end {
                                break;
                            }
                        }
                        _ => (),
                    };

                    entries.push(entry);

                    current = current + ENTRY_SIZE;
                }

                let table = Table {
                    header,
                    entries,
                };

                let result = Some((table, offset));
                Ok(result)
            }
            None => Ok(None),
        }
    }

    pub fn find_header<T: Read + Seek>(mut stream: &mut T) -> Result<Option<(Header, usize)>> {
        let offset = Self::find_offset(&mut stream)?;
        match offset {
            Some(offset) => {
                // Seek to offset
                stream.seek(SeekFrom::Start(offset as u64))?;

                // Read header into bytes
                let mut bytes = [0; HEADER_SIZE];
                stream.read_exact(&mut bytes)?;
                let header = Header::parse(&bytes)?;
                let result = Some((header, offset));
                Ok(result)
            }
            None => Ok(None),
        }
    }

    /// Find the offset of the DMA table.
    pub fn find_offset<T: Read + Seek>(stream: &mut T) -> Result<Option<usize>> {
        let align: u64 = TABLE_ALIGN.try_into().unwrap();
        let mut offset: u64 = 0;

        loop {
            stream.seek(SeekFrom::Start(offset))?;

            let mut magic = [0; 9];
            let amount = stream.read(&mut magic)?;

            if amount == 9 {
                if magic == TABLE_MAGIC {
                    return Ok(Some(offset.try_into().unwrap()));
                }
            } else {
                // Unable to read further, assume no table found
                return Ok(None);
            }

            offset = offset + align;
        }
    }
}
