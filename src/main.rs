use anyhow::{Context, Result};
use pico_args::Arguments;
use std::convert::TryInto;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

const HELP: &str = "\
deno-unpacker
USAGE:
  deno-unpacker [OPTIONS] --input PATH [INPUT]
FLAGS:
  -h, --help            Prints help information
OPTIONS:
  --input PATH          Sets a input path of file to unpack
  --output PATH         Sets an output path for unpacked source [default: 'source']
ARGS:
  <INPUT>
";

const MAGIC_TRAILER: &[u8; 8] = b"d3n0l4nd";

struct Args {
    input: String,
    output: String,
}

fn parse_args() -> Result<Args> {
    let mut pargs = Arguments::from_env();

    if pargs.contains(["-h", "--help"]) {
        print!("{}", HELP);
        std::process::exit(0);
    }

    let args = Args {
        input: pargs.value_from_str("--input")?,
        output: pargs
            .opt_value_from_str("--output")?
            .unwrap_or("source".to_string()),
    };

    Ok(args)
}

fn parse_executable(mut file: File) -> Result<Vec<u8>> {
    file.seek(SeekFrom::End(-24))?;
    let mut trailer = [0; 24];
    file.read_exact(&mut trailer)?;
    let (magic_trailer, rest) = trailer.split_at(8);
    if magic_trailer != MAGIC_TRAILER {
        return Err(anyhow::anyhow!(
            "This file doesn't deno executable, failed to parse trailer!"
        ));
    }

    let (bundle_pos, rest) = rest.split_at(8);
    let metadata_pos = rest;
    let bundle_pos = u64_from_bytes(bundle_pos)?;
    let metadata_pos = u64_from_bytes(metadata_pos)?;
    let bundle_len = metadata_pos - bundle_pos;

    let mut buffer = Vec::new();
    file.seek(SeekFrom::Start(bundle_pos))?;
    file.take(bundle_len).read_to_end(&mut buffer)?;
    Ok(buffer)
}

fn u64_from_bytes(arr: &[u8]) -> Result<u64> {
    let fixed_arr: &[u8; 8] = arr
        .try_into()
        .context("Failed to convert the buffer into a fixed-size array")?;
    Ok(u64::from_be_bytes(*fixed_arr))
}

fn unpack(args: Args) -> Result<()> {
    let input_file = File::open(args.input)?;
    let source = parse_executable(input_file)?;
    let source_file = format!("{}.ts", args.output);
    let path = Path::new(&source_file);
    match path.parent() {
        Some(path) => fs::create_dir_all(path)?,
        None => {}
    }
    let mut output_file = OpenOptions::new().write(true).create(true).open(path)?;
    Ok(output_file.write_all(&source)?)
}

fn main() {
    match parse_args() {
        Ok(args) => match unpack(args) {
            Ok(_) => {
                println!("Sources are successfully saved in the file!");
                std::process::exit(0);
            }
            Err(error) => {
                eprintln!("Failed to unpack file: {}", error);
            }
        },
        Err(error) => {
            eprintln!("Failed to parse args: {}", error);
        }
    };
    std::process::exit(1);
}