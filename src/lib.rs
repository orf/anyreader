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
