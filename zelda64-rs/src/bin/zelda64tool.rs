use anyhow::Result;
use clap::{App, Arg};
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
    let matches = App::new("zelda64tool")
        .author("saneki <s@neki.me>")
        .version("0.0.1")
        .about("Displays information about Zelda64 ROM files")
        .subcommand(
            App::new("decompress")
                .visible_alias("d")
                .about("Decompress a Zelda64 rom file")
                .arg(Arg::with_name("squeeze")
                    .short("s")
                    .long("squeeze")
                    .takes_value(false)
                    .help("Do not match decompressed addresses with virtual addresses."))
                .arg(Arg::with_name("input")
                    .required(true)
                    .help("Input rom file"))
                .arg(Arg::with_name("output")
                    .required(true)
                    .help("Output rom file"))
        )
        .subcommand(
            App::new("show")
                .about("Show details about a rom file")
                .arg(Arg::with_name("file")
                    .required(true)
                    .help("Zelda64 rom file"))
        )
        .get_matches();

    match matches.subcommand() {
        ("decompress", Some(matches)) => {
            let in_path = matches.value_of("input").unwrap();
            let (rom, _) = load_rom(&in_path)?;
            let squeeze = matches.is_present("squeeze");
            let options = decompress::Options::from(!squeeze);
            let mut dec_rom = decompress::decompress_with_options(&rom, &options)?;

            let out_path = matches.value_of("output").unwrap();
            let mut out_file = File::create(out_path)?;
            let written = dec_rom.write_with_update(&mut out_file)?;
            out_file.flush()?;
            println!("Wrote {:08X} bytes!", written);
        }
        ("show", Some(matches)) => {
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
        ("", None) => {
            println!("No subcommand was used");
        }
        _ => unreachable!(),
    }

    Ok(())
}
