use clap::{Arg, ArgMatches, Command};
use std::fs::{File, OpenOptions};
use std::io::{self, Seek, SeekFrom, Write};
use std::path::Path;
use std::process;
use thiserror::Error;

use n64rom::convert::{self, ConvertStatus};
use n64rom::header::Header;
use n64rom::rom::{Endianness, FileExt, Rom};
use n64rom::stream::Writer;
use n64rom::util::{self, FileSize, MEBIBYTE};

#[derive(Debug, Error)]
enum Error {
    #[error("{0}")]
    ConvertError(#[from] n64rom::convert::Error),
    /// Invalid CRC values.
    #[error("Bad CRC values, expected: ({0:#08X}, {1:#08X})")]
    CRCError(u32, u32),
    /// Error parsing Header.
    #[error("{0}")]
    HeaderError(#[from] n64rom::header::Error),
    /// IO error.
    #[error("{0}")]
    IOError(#[from] io::Error),
}

fn main() -> Result<(), Error> {
    let matches = Command::new("n64romtool")
        .author("saneki <s@neki.me>")
        .version("0.1.0")
        .about("Displays information about N64 ROM files")
        .subcommand(
            Command::new("show")
                .about("Show details about a rom file")
                .arg(Arg::new("file")
                    .required(true)
                    .help("Rom file"))
        )
        .subcommand(
            Command::new("check")
                .about("Verify whether or not the CRC values of a rom file are correct")
                .arg(Arg::new("file")
                    .required(true)
                    .help("Rom file"))
        )
        .subcommand(
            Command::new("convert")
                .about("Convert a rom file to a different byte order")
                .arg(Arg::new("in-place")
                    .short('i')
                    .long("in-place")
                    .takes_value(false)
                    .help("Modify input ROM file in-place."))
                .arg(Arg::new("ext")
                    .short('e')
                    .long("ext")
                    .takes_value(false)
                    .help("Update the ROM file extension for the corresponding byte order"))
                .arg(Arg::new("order")
                    .takes_value(true)
                    .possible_values(&["big", "little", "mixed"])
                    .required(true)
                    .help("Byte order to convert to"))
                .arg(Arg::new("input")
                    .required(true)
                    .help("Input rom file"))
                .arg(Arg::new("output")
                    .required_unless_present("in-place")
                    .help("Output rom file"))
        )
        .subcommand(
            Command::new("correct")
                .about("Correct the CRC values of a rom file")
                .arg(Arg::new("file")
                    .required(true)
                    .help("Rom file"))
        )
        .get_matches();

    match main_with_args(&matches) {
        Ok(()) => Ok(()),
        Err(Error::HeaderError(err)) => {
            println!("Error: {}, are you sure this is a rom file?", err);
            process::exit(1);
        }
        Err(Error::CRCError(crc1, crc2)) => {
            // Display default CRCError message
            println!("{}", Error::CRCError(crc1, crc2));
            process::exit(1);
        }
        Err(err) => {
            println!("Error: {}", err);
            process::exit(1);
        }
    }
}

fn load_rom(path: &str, with_body: bool) -> Result<(Rom, File), Error> {
    let in_path = Path::new(path);
    let mut file = File::open(in_path)?;
    let rom = Rom::read_with_body(&mut file, with_body)?;
    Ok((rom, file))
}

fn load_rom_rw(path: &str) -> Result<(Rom, File), Error> {
    let in_path = Path::new(path);
    let mut file = OpenOptions::new().read(true).write(true).open(in_path)?;
    let rom = Rom::read(&mut file)?;
    Ok((rom, file))
}

fn main_with_args(matches: &ArgMatches) -> Result<(), Error> {

    match matches.subcommand() {
        Some(("check", matches)) => {
            let path = matches.value_of("file").unwrap();
            let (rom, _) = load_rom(&path, true)?;

            let (result, crcs) = rom.check_crc();
            if result {
                println!("Correct!");
                Ok(())
            } else {
                Err(Error::CRCError(crcs.0, crcs.1))
            }
        }
        Some(("convert", matches)) => {
            // Get variables from arguments.
            let in_place = matches.is_present("in-place");
            let input = matches.value_of("input").unwrap();
            let order = match matches.value_of("order").unwrap() {
                "big" => Endianness::Big,
                "little" => Endianness::Little,
                "mixed" => Endianness::Mixed,
                _ => unreachable!(),
            };
            // Perform rom convert.
            let result = if in_place {
                // Update ROM file in-place.
                let use_ext = matches.is_present("ext");
                let (result, _) = convert::convert_rom_path_inplace(&input, order)?;
                if use_ext {
                    let ext = FileExt::from_endianness(order).unwrap();
                    util::update_file_extension(input, ext.as_str())?;
                }
                result
            } else {
                // Convert to separate output ROM file.
                let output = matches.value_of("output").unwrap();
                let (result, _) = convert::convert_rom_path(&input, &output, order)?;
                result
            };
            if matches!(result, ConvertStatus::AlreadyConverted) {
                println!("Rom file is already in {} byte order.", order);
            } else {
                println!("Done!");
            }
            Ok(())
        }
        Some(("correct", matches)) => {
            let path = matches.value_of("file").unwrap();
            let (mut rom, mut file) = load_rom_rw(&path)?;

            if rom.correct_crc() {
                println!("Rom CRC values are already correct!");
                Ok(())
            } else {
                file.seek(SeekFrom::Start(0))?;

                // Use a writer that respects the original byte order
                let mut writer = Writer::with_buffer_size(&mut file, rom.order(), Header::SIZE);
                rom.header.write(&mut writer)?;
                writer.flush()?;

                println!("Corrected!");
                Ok(())
            }
        }
        Some(("show", matches)) => {
            // Read rom with only head (header & IPL3)
            let path = matches.value_of("file").unwrap();
            let (rom, file) = load_rom(&path, false)?;

            // For efficiency, instead of reading all data to determine rom size, check file metadata
            let metadata = file.metadata()?;
            let filesize = FileSize::from(metadata.len(), MEBIBYTE);

            // Show size text in MiB
            let sizetext = match filesize {
                FileSize::Float(value) => {
                    format!("{:.*} MiB", 1, value)
                }
                FileSize::Int(value) => {
                    format!("{} MiB", value)
                }
            };

            println!("{}", rom);
            println!("  Rom Size: {}", &sizetext);

            Ok(())
        }
        None => {
            println!("No subcommand was used");
            Ok(())
        }
        _ => unreachable!(),
    }
}
