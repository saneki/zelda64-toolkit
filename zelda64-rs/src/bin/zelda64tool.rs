use anyhow::Result;
use clap::{Arg, Command};
use n64rom::rom::HEAD_SIZE;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use zelda64::decompress;
use zelda64::rom::Rom;

fn load_rom(path: &str) -> Result<(Rom, File)> {
    let in_path = Path::new(path);
    let mut file = File::open(in_path)?;
    let rom = Rom::read(&mut file)?;
    Ok((rom, file))
}

fn main() -> Result<()> {
    let matches = Command::new("zelda64tool")
        .author("saneki <s@neki.me>")
        .version("0.0.1")
        .about("Displays information about Zelda64 ROM files")
        .subcommand(
            Command::new("decompress")
                .visible_alias("d")
                .about("Decompress a Zelda64 rom file")
                .arg(Arg::new("squeeze")
                    .short('s')
                    .long("squeeze")
                    .takes_value(false)
                    .help("Do not match decompressed addresses with virtual addresses."))
                .arg(Arg::new("input")
                    .required(true)
                    .help("Input rom file"))
                .arg(Arg::new("output")
                    .required(true)
                    .help("Output rom file"))
        )
        .subcommand(
            Command::new("show")
                .about("Show details about a rom file")
                .arg(Arg::new("file")
                    .required(true)
                    .help("Zelda64 rom file"))
        )
        .get_matches();

    match matches.subcommand() {
        Some(("decompress", matches)) => {
            let in_path = matches.value_of("input").unwrap();
            let (rom, _) = load_rom(&in_path)?;
            let squeeze = matches.is_present("squeeze");
            let mut dec_rom = decompress::decompress(&rom, !squeeze)?;

            let out_path = matches.value_of("output").unwrap();
            let mut out_file = File::create(out_path)?;
            let written = dec_rom.write_with_update(&mut out_file)?;
            out_file.flush()?;
            println!("Wrote {:08X} bytes!", written);
        }
        Some(("show", matches)) => {
            let path = matches.value_of("file").unwrap();
            let (rom, _) = load_rom(&path)?;

            match &rom.table {
                Some(table) => {
                    // Factor in size of N64 rom header
                    let offset = (table.address as usize) + HEAD_SIZE;

                    println!("Table: 0x{:08X}", offset);
                    println!("{}", table);
                },
                None => println!("No table?")
            }
        }
        None => {
            println!("No subcommand was used");
        }
        _ => unreachable!(),
    }

    Ok(())
}
