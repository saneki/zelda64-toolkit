use n64rom::rom::Rom as N64Rom;
use std::io::{self, Cursor, Read, Seek, SeekFrom, Write};
use std::ops::Range;
use thiserror::Error;

use crate::dma::{self, Entry, Table};

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
    pub table: Option<Table>,
}

impl Rom {
    pub fn from(rom: N64Rom, table: Option<Table>) -> Self {
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
            Some((table, _)) => Rom::from(n64rom, Some(table)),
            None => Rom::from(n64rom, None),
        };

        Ok(rom)
    }

    pub fn slice(&self, entry: &Entry) -> &[u8] {
        let (range, _) = entry.range_usize();
        let range = range.unwrap(); // TODO: Return Result type instead of unwrap range.
        &self.rom.full()[range]
    }

    pub fn update(&mut self) -> Result<()> {
        self.update_table_data()?;
        // Correct CRC values
        self.rom.correct_crc();
        Ok(())
    }

    fn update_table_data(&mut self) -> Result<()> {
        match &self.table {
            Some(table) => {
                let offset = table.address as usize;
                let range = Range { start: offset, end: offset + table.size() };
                let mut slice = &mut self.rom.full_mut()[range];
                table.write(&mut slice)?;
                Ok(())
            }
            None => Ok(()),
        }
    }

    pub fn write<T: Write>(&mut self, mut writer: &mut T) -> io::Result<usize> {
        self.rom.write(&mut writer, None)
    }

    pub fn write_with_update<T: Seek + Write>(&mut self, mut writer: &mut T) -> Result<usize> {
        self.update()?;
        let written = self.write(&mut writer)?;
        Ok(written)
    }
}
