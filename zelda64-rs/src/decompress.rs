use n64rom::rom::Rom as N64Rom;
use std::io::Cursor;
use std::ops::Range;
use thiserror::Error;
use yaz0::inflate::Yaz0Archive;

use crate::dma::{self, Entry, EntryType, Table};
use crate::rom::{self, Rom};
use crate::util::{self, ConvertRangeExt};

/// Decompressed rom capacity is 64 MiB.
const ROM_CAPACITY: usize = 1024 * 1024 * 64;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    DmaError(#[from] dma::Error),
    #[error("{0}")]
    RomError(#[from] rom::Error),
    #[error("Virtual address out-of-range for output slice: (0x{:8X}, 0x{:8X})", .0.start, .0.end)]
    OutOfRangeError(Range<u32>),
    #[error("Yaz0 decompression error: {0}")]
    Yaz0Error(#[from] ::yaz0::Error),
}

/// Decompression options.
pub struct Options {
    /// Whether or not to match virtual addresses.
    matching: bool,
}

impl Options {
    /// Get the default set of decompression `Options`.
    pub fn default() -> Self {
        Self::from(true)
    }

    pub fn from(matching: bool) -> Self {
        Self { matching }
    }
}

/// Decompress `dmadata` filesystem in ROM with default options.
pub fn decompress(rom: &Rom) -> Result<Rom, Error> {
    let options = Options::default();
    decompress_rom(rom, &options)
}

/// Decompress `dmadata` filesystem in ROM.
pub fn decompress_rom(rom: &Rom, options: &Options) -> Result<Rom, Error> {
    let n64rom = &rom.rom;
    let mut data = vec![0; ROM_CAPACITY];
    let table = rom.table.as_ref().unwrap();
    let mut entries = Vec::with_capacity(table.entries.len());
    let mut offset = 0;

    for entry in &table.entries {
        let (virt, range, kind) = entry.validate()?;
        match range {
            Some(_) => {
                let input = rom.slice(&entry);
                // Either use virtual addresses for output slice, or begin where last slice ended.
                let outrange = if options.matching {
                    virt.clone()
                } else {
                    let length = util::align16(virt.len() as u32);
                    let result = Range { start: offset, end: offset + length };
                    offset += length;
                    result
                };
                let mut output = data.get_mut(outrange.to_usize()).ok_or(Error::OutOfRangeError(outrange.clone()))?;
                let entry = Entry::from_uncompressed(virt.start, virt.end, outrange.start);
                match kind {
                    EntryType::Compressed => {
                        // Decompress Yaz0-compressed file data.
                        let mut cursor = Cursor::new(input);
                        let mut archive = Yaz0Archive::new(&mut cursor)?;
                        archive.decompress_into(&mut output)?;
                    }
                    EntryType::Decompressed => {
                        // Direct copy as file data is not compressed.
                        output.copy_from_slice(input);
                    }
                    _ => unreachable!()
                }
                entries.push(entry);
            }
            _ => entries.push(entry.clone())
        }
    }

    let new_table = Table::from(table.address, entries);
    let new_n64rom = N64Rom::from(n64rom.header, n64rom.ipl3, data, n64rom.order());
    let new_rom = Rom::from(new_n64rom, Some(new_table));

    Ok(new_rom)
}
