use anyreader::{iterate_archive, recursive_read};
use clap::Parser;
use clio::*;
use std::io;
use std::io::{BufReader, BufWriter, Read, Seek, Write};
use std::path::Path;
use tracing::info;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// A simple CLI tool to read files and directories and write them to a tar archive.
#[derive(Parser)]
#[command(version)]
struct Args {
    /// Input files to read. Can be raw, compressed or archive files. Use `-` for stdin.
    #[clap(value_parser, required = true)]
    input: Vec<Input>,
    /// Output file to write the tar archive to. Use `-` for stdout.
    #[clap(value_parser)]
    output: Output,
    /// Don't recurse into nested archives or decompress entry contents.
    #[clap(long, short = 's')]
    shallow: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let mut builder = tar::Builder::new(BufWriter::new(args.output));

    for input in args.input {
        handle_reader(
            &input.path().clone(),
            BufReader::new(input),
            &mut builder,
            args.shallow,
        )?;
    }

    builder.finish()?;
    Ok(())
}

fn handle_reader(
    path: &Path,
    reader: impl Read,
    builder: &mut tar::Builder<impl Write + Seek>,
    shallow: bool,
) -> io::Result<()> {
    let mut callback = |item: anyreader::FileItem<&mut dyn Read>| {
        if !item.kind.is_file() {
            return Ok(());
        }
        let mut header = tar::Header::new_gnu();
        let mut entry = builder.append_writer(&mut header, &item.path)?;
        std::io::copy(item.reader, &mut entry)?;
        info!("Wrote {:?}", item.path);
        Ok(())
    };

    if shallow {
        iterate_archive(reader, &mut callback)?;
    } else {
        recursive_read(path, reader, &mut callback)?;
    }
    Ok(())
}
