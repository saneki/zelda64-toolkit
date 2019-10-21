use n64rom::rom::Rom as N64Rom;
use n64rom::rom::HEAD_SIZE;
use std::convert::TryInto;
use std::io::{Cursor, Read, Seek, SeekFrom};
use yaz0::inflate::Yaz0Archive;

use crate::dma::{Entry, EntryType, Table};
use crate::rom::{Error, Rom};

/// Decompressed rom capacity is 64 MiB.
const ROM_CAPACITY: usize = 1024 * 1024 * 64;

pub struct Decompressor<'d> {
    rom: &'d Rom,
}

impl<'d> Decompressor<'d> {
    pub fn from(rom: &'d Rom) -> Self {
        Self {
            rom
        }
    }

    pub fn decompress(&mut self) -> Result<Rom, Error> {
        let n64rom = &self.rom.rom;
        let data = &n64rom.data;
        let mut cursor = Cursor::new(data);

        let head_size: u32 = HEAD_SIZE.try_into().unwrap();

        // Current physical offset.
        let mut current: u32 = 0;
        // Updated entry structs.
        let mut entries: Vec<Entry> = Vec::new();
        // Output vector.
        let mut result = Vec::new();

        let (table, offset) = self.rom.table.as_ref().unwrap();
        for entry in &table.entries {
            let (virt, phys, kind) = entry.validate()?;
            match phys {
                Some(phys) => {
                    match kind {
                        EntryType::Compressed => {
                            let entry = Entry::from_decompressed(virt, current);
                            entries.push(entry);

                            // Todo: If still before 0x1000, error?
                            let start = phys.start - head_size;

                            cursor.seek(SeekFrom::Start(start.into()))?;
                            // Todo: Make yaz0::error::Error public, unwrapping for now.
                            let mut archive = Yaz0Archive::new(&mut cursor).unwrap();
                            let data = archive.decompress().unwrap();

                            // Todo: Check that `data.len() == virt.len()`?
                            //if data.len() != virt.len() {
                            //    println!("WARN: Length mismatch: {}", &entry);
                            //}

                            // Write to result buffer.
                            result.extend(&data);

                            let datalen: u32 = data.len().try_into().unwrap();
                            current = current + datalen;
                        }
                        EntryType::Decompressed => {
                            let entry = Entry::from_decompressed(virt, current);
                            entries.push(entry);

                            // Ignore the first 0x1000 bytes of rom (N64 rom header & IPL).
                            // This should only ever apply to the very first entry.
                            let (start, length) = if phys.start < head_size {
                                current = current + head_size;
                                let length = (phys.end - phys.start) - head_size;
                                let length: usize = length.try_into().unwrap();
                                (0u32, length)
                            } else {
                                (phys.start - head_size, phys.len())
                            };

                            cursor.seek(SeekFrom::Start(start.into()))?;
                            let mut data = vec![0; length];
                            cursor.read_exact(&mut data)?;

                            // Write to result buffer
                            result.extend(&data);

                            let datalen: u32 = data.len().try_into().unwrap();
                            current = current + datalen;
                        },
                        _ => unreachable!(),
                    }
                }
                _ => entries.push(entry.clone()),
            }
        }

        result.resize(ROM_CAPACITY - HEAD_SIZE, 0);

        let new_table = Table::from(table.header.clone(), entries);
        let new_n64rom = N64Rom::from(n64rom.header, n64rom.ipl3, result, n64rom.order().clone());
        let new_rom = Rom::from(new_n64rom, Some((new_table, *offset)));

        Ok(new_rom)
    }
}
