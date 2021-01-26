use n64rom::rom::Rom as N64Rom;
use std::convert::TryInto;
use std::io::{self, Cursor, Read, Seek, SeekFrom, Write};
use thiserror::Error;

use crate::dma::{self, Table};

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    DMAError(#[from] dma::Error),
    #[error("{0}")]
    HeaderError(#[from] n64rom::header::Error),
    #[error("{0}")]
    IOError(#[from] io::Error),
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
        let mut cursor = Cursor::new(n64rom.full());
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
