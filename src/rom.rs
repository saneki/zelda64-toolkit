use failure::Fail;
use n64rom::header::HeaderError;
use n64rom::rom::Rom as N64Rom;
use std::convert::TryInto;
use std::io::{self, Cursor, Read, Seek, SeekFrom, Write};

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

type Result<T> = ::std::result::Result<T, Error>;

/// Zelda64 rom.
pub struct Rom {
    /// Underlying N64 rom.
    pub rom: N64Rom,
    pub table: Option<(Table, usize)>,
}

impl Rom {
    pub fn from(rom: N64Rom, table: Option<(Table, usize)>) -> Self {
        Self {
            rom,
            table,
        }
    }

    pub fn patch(&mut self, offset: u64, bytes: &[u8]) -> io::Result<usize> {
        let mut cursor = Cursor::new(self.rom.data_mut());
        cursor.seek(SeekFrom::Start(offset))?;
        cursor.write(bytes)
    }

    pub fn read<T: Read>(mut reader: &mut T) -> Result<Self> {
        let n64rom = N64Rom::read(&mut reader)?;

        // Wrap data in cursor and search for Table structure
        let mut cursor = Cursor::new(n64rom.data());
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

    pub fn update(&mut self) -> Result<()> {
        self.update_table_data()?;
        // Correct CRC values
        self.rom.correct_crc();
        Ok(())
    }

    fn update_table_data(&mut self) -> Result<()> {
        match &self.table {
            Some((table, offset)) => {
                let mut cursor = Cursor::new(Vec::new());
                let offset: u64 = (*offset).try_into().unwrap();
                table.write(&mut cursor)?;
                self.patch(offset, cursor.get_ref())?;
                Ok(())
            }
            None => Ok(()),
        }
    }

    pub fn write<T: Write>(&self, mut writer: &mut T) -> io::Result<usize> {
        self.rom.write(&mut writer, None)
    }

    pub fn write_with_update<T: Seek + Write>(&mut self, mut writer: &mut T) -> Result<usize> {
        self.update()?;
        let written = self.write(&mut writer)?;
        Ok(written)
    }
}
