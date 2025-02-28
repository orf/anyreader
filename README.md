# Anyreader

A library for reading streams of compressed and uncompressed data without knowing the format in advance.

You can use this to recursively read raw data from potentially compressed streams that may contain archives.

# Installation:

`cargo add anyreader`

# Example:

The code below will read the contents of a tar.gz archive and print the size of each file. Any archives or
compressed files _within_ the archive will also be recursively read:

```rust
use anyreader::{recursive_read, FileItem};
use std::path::Path;
use std::fs::File;
use std::io::{self, BufReader};

fn main() -> io::Result<()> {
    let path = Path::new("tests/data/archive.tar.gz");
    let reader = BufReader::new(File::open(&path)?);

    recursive_read(path, reader, &mut |mut item: FileItem<_>| {
        println!("Found file: {}", item.path.display());
        let size = io::copy(&mut item.reader, &mut io::sink())?;
        println!("Size: {}", size);
        Ok(())
    })?;

    Ok(())
}
```

# CLI

The `anyreader` crate also comes with a CLI tool that can be used to read files from the command line. It will 
recursively read any archives or compressed files within the input files, and write the decompressed, flattened 
contents to the output file in a `tar` format.

This requires the `cli` feature: `cargo install anyreader -F cli`

```sh
anyreader tests/data/archive.tar.gz tests/data/archive-2.zip output.tar
```

```sh
A simple CLI tool to read files and directories and write them to a tar archive

Usage: anyreader <INPUT>... <OUTPUT>

Arguments:
  <INPUT>...  Input files to read. Can be raw, compressed or archive files. Use `-` for stdin
  <OUTPUT>    Output file to write the tar archive to. Use `-` for stdout

Options:
  -h, --help     Print help
  -V, --version  Print version
```