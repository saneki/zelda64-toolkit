use failure::Fail;
use n64rom::header::HeaderError;
use n64rom::rom::Rom as N64Rom;
use std::io::{self, Cursor, Read};

use crate::dma::{self, Table};

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "{}", _0)]
    DMAError(#[cause] dma::Error),

    #[fail(display = "{}", _0)]
    HeaderError(#[cause] HeaderError),

    #[fail(display = "{}", _0)]
    IOError(#[cause] io::Error),
}

impl From<dma::Error> for Error {
    fn from(e: dma::Error) -> Self {
        Error::DMAError(e)
    }
}

impl From<HeaderError> for Error {
    fn from(e: HeaderError) -> Self {
        Error::HeaderError(e)
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IOError(e)
    }
}

/// Zelda64 rom.
pub struct Rom {
    /// Underlying N64 rom.
    pub rom: N64Rom,
    pub table: Option<(Table, usize)>,
}

impl Rom {
    pub fn read<T: Read>(mut reader: &mut T) -> Result<Self, Error> {
        let n64rom = N64Rom::read(&mut reader)?;

        // Wrap data in cursor and search for Table structure
        let mut cursor = Cursor::new(&n64rom.data);
        let result = Table::find(&mut cursor)?;
        let rom = match result {
            Some((table, offset)) => {
                Rom {
                    rom: n64rom,
                    table: Some((table, offset)),
                }
            }
            None => {
                Rom {
                    rom: n64rom,
                    table: None,
                }
            }
        };

        Ok(rom)
    }
}
