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

#[derive(Debug)]
pub struct FileItem<T: Read> {
    pub path: PathBuf,
    pub reader: T,
    pub kind: FileKind,
}

pub fn recursive_read<F>(path: &Path, mut reader: impl Read, callback: &mut F) -> io::Result<()>
where
    F: FnMut(FileItem<&mut dyn Read>) -> io::Result<()>,
{
    read_recursive_inner(path, FileKind::File, &mut reader as &mut dyn Read, callback)?;
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
        read_recursive_inner(path.join(x.path).as_path(), x.kind, reader, callback)?;
    }
    Ok(())
}

fn read_recursive_inner<F>(
    path: &Path,
    kind: FileKind,
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
        }),
        ContainerKind::Stream(StreamKind::Compressed(mut c)) => {
            read_recursive_inner(path, kind, &mut c as &mut dyn Read, callback)
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
        })?;
    }
    Ok(())
}
