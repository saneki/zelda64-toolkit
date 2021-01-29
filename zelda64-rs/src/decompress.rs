use n64rom::rom::Rom as N64Rom;
use std::io::Cursor;
use std::ops::Range;
use thiserror::Error;
use yaz0::inflate::Yaz0Archive;

use crate::dma::{self, Entry, EntryType, Table};
use crate::rom::{self, Rom};
use crate::util::ConvertRangeExt;

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
}

/// Decompress `dmadata` filesystem in ROM.
pub fn decompress_rom(rom: &Rom) -> Result<Rom, Error> {
    let n64rom = &rom.rom;
    let mut data = vec![0; ROM_CAPACITY];
    let table = rom.table.as_ref().unwrap();
    let mut entries = Vec::with_capacity(table.entries.len());

    for entry in &table.entries {
        let (virt, range, kind) = entry.validate()?;
        match range {
            Some(_) => {
                // Get decompressed length and create new Entry.
                let input = rom.slice(&entry);
                let mut output = data.get_mut(virt.to_usize()).ok_or(Error::OutOfRangeError(virt.clone()))?;
                let entry = Entry::from_uncompressed(virt.start, virt.end, virt.start);
                match kind {
                    EntryType::Compressed => {
                        // Decompress Yaz0-compressed file data.
                        let mut cursor = Cursor::new(input);
                        let mut archive = Yaz0Archive::new(&mut cursor).unwrap();
                        archive.decompress_into(&mut output).unwrap();
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
