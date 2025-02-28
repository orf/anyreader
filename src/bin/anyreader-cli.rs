use anyreader::read_recursive;
use clap::Parser;
use clio::*;
use std::io::{BufReader, BufWriter, Read, Seek, Write};
use std::path::Path;
use tracing::info;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[derive(Parser)]
struct Args {
    #[clap(value_parser, required = true)]
    input: Vec<Input>,
    #[clap(value_parser)]
    output: Output,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let mut builder = tar::Builder::new(BufWriter::new(args.output));

    for input in args.input {
        handle_reader(&input.path().clone(), BufReader::new(input), &mut builder);
    }

    builder.finish()?;
    Ok(())
}

fn handle_reader(path: &Path, reader: impl Read, builder: &mut tar::Builder<impl Write + Seek>) {
    read_recursive(path, reader, &mut |item| {
        if !item.kind.is_file() {
            return;
        }
        let mut header = tar::Header::new_gnu();
        let mut entry = builder.append_writer(&mut header, &item.path).unwrap();
        std::io::copy(item.reader, &mut entry).unwrap();
        info!("Wrote {:?}", item.path);
    });
}
