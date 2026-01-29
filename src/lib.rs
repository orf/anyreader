#![doc = include_str!("../README.md")]

use std::io;
use std::io::Read;
use std::path::{Path, PathBuf};

mod container;
mod peekable;
mod stream;

pub use crate::container::{ArchiveKind, Container, ContainerKind, Items};
pub use crate::stream::CompressionKind;
pub use crate::stream::StreamKind;

#[derive(Debug, strum::EnumIs)]
pub enum FileKind {
    File,
    Directory,
    Other,
}

/// Represents what is known about the size of a file or entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SizeHint {
    /// The exact uncompressed size is known (e.g., from archive entry headers).
    Exact(u64),
    /// Only the compressed size is known (after transparent decompression).
    CompressedSize(u64),
    /// No size information is available.
    #[default]
    Unknown,
}

impl SizeHint {
    /// Returns the exact size if known.
    pub fn exact(&self) -> Option<u64> {
        match self {
            SizeHint::Exact(n) => Some(*n),
            _ => None,
        }
    }

    /// Returns the compressed size if that's what is known.
    pub fn compressed_size(&self) -> Option<u64> {
        match self {
            SizeHint::CompressedSize(n) => Some(*n),
            _ => None,
        }
    }

    /// Returns any known size (exact or compressed).
    pub fn any_known(&self) -> Option<u64> {
        match self {
            SizeHint::Exact(n) | SizeHint::CompressedSize(n) => Some(*n),
            SizeHint::Unknown => None,
        }
    }

    /// Returns true if the size is exactly known.
    pub fn is_exact(&self) -> bool {
        matches!(self, SizeHint::Exact(_))
    }

    /// Returns true if the size is unknown.
    pub fn is_unknown(&self) -> bool {
        matches!(self, SizeHint::Unknown)
    }
}

#[derive(Debug)]
pub struct FileItem<T: Read> {
    pub path: PathBuf,
    pub reader: T,
    pub kind: FileKind,
    pub size_hint: SizeHint,
}

pub fn recursive_read<F>(path: &Path, mut reader: impl Read, callback: &mut F) -> io::Result<()>
where
    F: FnMut(FileItem<&mut dyn Read>) -> io::Result<()>,
{
    read_recursive_inner(
        path,
        FileKind::File,
        SizeHint::Unknown,
        &mut reader as &mut dyn Read,
        callback,
    )?;
    Ok(())
}

fn handle_container<F>(path: &Path, mut archive: impl Container, callback: &mut F) -> io::Result<()>
where
    F: FnMut(FileItem<&mut dyn Read>) -> io::Result<()>,
{
    let mut items = archive.items()?;
    while let Some(x) = items.next_item() {
        let mut x = x?;
        let reader = &mut x.reader as &mut dyn Read;
        read_recursive_inner(
            path.join(x.path).as_path(),
            x.kind,
            x.size_hint,
            reader,
            callback,
        )?;
    }
    Ok(())
}

fn read_recursive_inner<F>(
    path: &Path,
    kind: FileKind,
    size_hint: SizeHint,
    reader: &mut dyn Read,
    callback: &mut F,
) -> io::Result<()>
where
    F: FnMut(FileItem<&mut dyn Read>) -> io::Result<()>,
{
    let container = ContainerKind::from_reader(reader)?;
    match container {
        ContainerKind::Stream(StreamKind::Raw(mut r)) => callback(FileItem {
            path: path.to_path_buf(),
            reader: &mut r as &mut dyn Read,
            kind,
            size_hint,
        }),
        ContainerKind::Stream(StreamKind::Compressed(mut c)) => {
            // When decompressing, convert Exact to CompressedSize
            let new_hint = match size_hint {
                SizeHint::Exact(n) => SizeHint::CompressedSize(n),
                other => other,
            };
            read_recursive_inner(path, kind, new_hint, &mut c as &mut dyn Read, callback)
        }
        ContainerKind::Archive(ArchiveKind::Tar(r)) => handle_container(path, r, callback),
        ContainerKind::Archive(ArchiveKind::Zip(r)) => handle_container(path, r, callback),
    }
}

/// Unwraps compression layers and iterates archive entries without recursion.
///
/// This function decompresses outer layers (gzip, zstd, bzip2, xz) to reach the archive,
/// then iterates archive entries (tar or zip) once. Unlike [`recursive_read`], it does NOT
/// recurse into nested archives or decompress entry contents - entries are returned with
/// their raw bytes.
///
/// Returns an error if the input is not an archive after decompression.
pub fn iterate_archive<R, F>(mut reader: R, mut callback: F) -> io::Result<()>
where
    R: Read,
    F: FnMut(FileItem<&mut dyn Read>) -> io::Result<()>,
{
    iterate_archive_inner(&mut reader as &mut dyn Read, &mut callback)
}

fn iterate_archive_inner<F>(reader: &mut dyn Read, callback: &mut F) -> io::Result<()>
where
    F: FnMut(FileItem<&mut dyn Read>) -> io::Result<()>,
{
    let container = ContainerKind::from_reader(reader)?;
    match container {
        // Raw data after decompression - not an archive
        ContainerKind::Stream(StreamKind::Raw(_)) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "input is not an archive",
        )),
        // Compressed - recurse through compression layer
        ContainerKind::Stream(StreamKind::Compressed(mut c)) => {
            iterate_archive_inner(&mut c as &mut dyn Read, callback)
        }
        // Archive found - iterate entries without recursively decompressing
        ContainerKind::Archive(ArchiveKind::Tar(mut r)) => iterate_entries(&mut r, callback),
        ContainerKind::Archive(ArchiveKind::Zip(mut r)) => iterate_entries(&mut r, callback),
    }
}

fn iterate_entries<F>(archive: &mut impl Container, callback: &mut F) -> io::Result<()>
where
    F: FnMut(FileItem<&mut dyn Read>) -> io::Result<()>,
{
    let mut items = archive.items()?;
    while let Some(item) = items.next_item() {
        let mut item = item?;
        callback(FileItem {
            path: item.path,
            reader: &mut item.reader as &mut dyn Read,
            kind: item.kind,
            size_hint: item.size_hint,
        })?;
    }
    Ok(())
}
