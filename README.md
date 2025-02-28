# Anyreader

A library for reading streams of compressed and uncompressed data without knowing the format in advance.

You can use this to recursively read raw data from potentially compressed streams that may contain archives.

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