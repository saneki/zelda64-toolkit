use std::fmt;
use std::io::{self, Cursor, Read, Write};
use thiserror::Error;

use crate::header::Header;
use crate::ipl3::{IPL3, IPL_SIZE};
use crate::stream::{Reader, Writer};
use crate::util::{FileSize, MEBIBYTE};

/// Total size of rom header and IPL3. This will be the file offset where data begins.
pub const HEAD_SIZE: usize = Header::SIZE + IPL_SIZE;

/// Maximum expected rom size (64 MiB).
pub const MAX_SIZE: usize = 1024 * 1024 * 64;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    IOError(#[from] io::Error),
    #[error("{0}")]
    HeaderError(#[from] crate::header::Error),
    #[error("Unsupported endianness for this operation: {0}")]
    UnsupportedEndianness(Endianness),
}

#[derive(Clone, Copy, Debug, PartialEq)]
/// Convenience wrapper enum around the separate Swap endianness enums.
pub enum Endianness {
    Big,
    Little,
    Mixed,
}

impl Endianness {
    pub fn from_file_ext(ext: FileExt) -> Endianness {
        match ext {
            FileExt::N64 => Endianness::Little,
            FileExt::V64 => Endianness::Mixed,
            FileExt::Z64 => Endianness::Big,
        }
    }
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FileExt {
    N64,
    V64,
    Z64,
}

impl FileExt {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::N64 => "n64",
            Self::V64 => "v64",
            Self::Z64 => "z64",
        }
    }

    pub fn from_endianness(e: Endianness) -> Option<FileExt> {
        // NOTE: Using Option in anticipation of wordswapped Endianness, which would not have a file extension.
        match e {
            Endianness::Big => Some(FileExt::Z64),
            Endianness::Little => Some(FileExt::N64),
            Endianness::Mixed => Some(FileExt::V64),
        }
    }
}

impl fmt::Display for FileExt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Clone)]
pub struct Rom {
    pub header: Header,
    pub ipl3: IPL3,
    /// Full Rom image data.
    pub image: Vec<u8>,
    /// Byte order (endianness) of rom file.
    order: Endianness,
}

impl fmt::Display for Rom {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.header)?;
        write!(formatter, "  IPL3: {}", self.ipl3)?;
        write!(formatter, "  Byte Order: {}", self.order)?;
        // Only show rom size if we have data.
        if self.image.len() > HEAD_SIZE {
            let filesize = FileSize::from(self.len() as u64, MEBIBYTE);
            match filesize {
                FileSize::Float(value) => {
                    write!(formatter, "  Rom Size: {:.*} MiB", 1, value)?;
                }
                FileSize::Int(value) => {
                    write!(formatter, "  Rom Size: {} MiB", value)?;
                }
            }
        }
        Ok(())
    }
}

impl Rom {
    /// Calculate CRC values from `Rom` data and compare against CRC values in the `Header`.
    pub fn check_crc(&self) -> (bool, (u32, u32)) {
        let crcs = self.header.crcs();
        let calc = self.ipl3.compute_crcs(&self.image[HEAD_SIZE..], &[]);
        let result = crcs == calc;
        (result, calc)
    }

    /// Correct the CRC values in the header.
    pub fn correct_crc(&mut self) -> bool {
        let (result, (calc1, calc2)) = self.check_crc();
        match result {
            true => result,
            false => {
                // Update the header CRC fields
                self.header.crc1 = calc1;
                self.header.crc2 = calc2;

                result
            }
        }
    }

    /// Get slice of `Rom` image data, not including header or `IPL3`.
    pub fn data(&self) -> &[u8] {
        &self.image[HEAD_SIZE..]
    }

    /// Get slice of `Rom` image data as mutable, not including header or `IPL3`.
    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.image[HEAD_SIZE..]
    }

    /// Create `Rom` from a raw image without copying. Requires image data to be in big-endian format.
    pub fn from_image(image: Vec<u8>) -> Result<Self, Error> {
        let mut head = &image[..HEAD_SIZE];
        // Read header & infer endianness.
        let (header, order) = Header::read_ordered(&mut head)?;
        if order == Endianness::Big {
            let ipl3 = IPL3::read(&mut head)?;
            Ok(Rom::from(header, ipl3, image, order))
        } else {
            Err(Error::UnsupportedEndianness(order))
        }
    }

    /// Create `Rom` from fields.
    pub fn from(header: Header, ipl3: IPL3, image: Vec<u8>, order: Endianness) -> Self {
        Self {
            header,
            ipl3,
            image,
            order,
        }
    }

    /// Get slice of full `Rom` image data.
    pub fn full(&self) -> &[u8] {
        &self.image[..]
    }

    /// Get slice of full `Rom` image data as mutable.
    pub fn full_mut(&mut self) -> &mut [u8] {
        &mut self.image[..]
    }

    /// Get the `Endianness` of the parsed `Rom` data.
    pub fn order(&self) -> Endianness {
        self.order
    }

    /// Read `Rom` with all data.
    pub fn read<T: Read>(mut reader: &mut T) -> Result<Self, crate::header::Error> {
        Self::read_with_body(&mut reader, true)
    }

    /// Read `Rom`.
    pub fn read_with_body<T: Read>(mut reader: &mut T, read_body: bool) -> Result<Self, crate::header::Error> {
        // Read header & infer endianness
        let (header, order) = Header::read_ordered(&mut reader)?;

        // Create new reader based on endianness, read remaining with it
        let mut reader = Reader::from(&mut reader, order);
        let ipl3 = IPL3::read(&mut reader)?;

        // Read rom data into buffer.
        let mut image = Vec::new();
        header.write(&mut image)?;
        image.extend(ipl3.get_ipl());
        // Read remaining data if specified.
        if read_body {
            reader.read_to_end(&mut image)?;
        }
        let image = image;

        let rom = Self {
            header,
            ipl3,
            image,
            order,
        };

        Ok(rom)
    }

    /// Flush `Header` and `IPL3` to underlying buffer.
    pub fn flush(&mut self) -> io::Result<usize> {
        let slice = &mut self.image[..HEAD_SIZE];
        let mut cursor = Cursor::new(slice);
        let mut written = self.header.write(&mut cursor)?;
        written += self.ipl3.write(&mut cursor)?;
        Ok(written)
    }

    /// Write `Rom` data to writer.
    pub fn write_raw<T: Write>(&self, writer: &mut T, endianness: Option<Endianness>) -> io::Result<usize> {
        let order = endianness.unwrap_or(self.order);
        // Todo: Compare total amount written to expected length
        Writer::write_all(writer, &self.image, order)
    }

    /// Write `Rom` data to writer after flushing `Header` and `IPL3` to underlying buffer.
    pub fn write<T: Write>(&mut self, writer: &mut T, endianness: Option<Endianness>) -> io::Result<usize> {
        self.flush()?;
        self.write_raw(writer, endianness)
    }

    /// Get full length of `Rom` data.
    pub fn len(&self) -> usize {
        self.image.len()
    }

    /// Whether or not the `Rom` data is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
