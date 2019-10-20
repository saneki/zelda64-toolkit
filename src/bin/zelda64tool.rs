use clap::{App, Arg};
use n64rom::rom::HEAD_SIZE;
use std::fs::File;
use std::path::Path;
use zelda64::rom::{Error, Rom};

fn load_rom(path: &str) -> Result<(Rom, File), Error> {
    let in_path = Path::new(path);
    let mut file = File::open(in_path)?;
    let rom = Rom::read(&mut file)?;
    Ok((rom, file))
}

fn main() -> Result<(), Error> {
    let matches = App::new("zelda64tool")
        .author("saneki <s@neki.me>")
        .version("0.0.1")
        .about("Displays information about Zelda64 ROM files")
        .subcommand(
            App::new("show")
                .about("Show details about a rom file")
                .arg(Arg::with_name("file")
                    .required(true)
                    .help("Zelda64 rom file"))
        )
        .get_matches();

    match matches.subcommand() {
        ("show", Some(matches)) => {
            let path = matches.value_of("file").unwrap();
            let (rom, _) = load_rom(&path)?;

            match &rom.table {
                Some((table, offset)) => {
                    // Factor in size of N64 rom header
                    let offset = offset + HEAD_SIZE;

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
