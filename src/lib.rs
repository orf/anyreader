use std::io::Read;
use std::path::Path;

mod container;
mod peekable;
mod stream;

pub use crate::container::{ArchiveKind, Container, ContainerKind, Items};
use crate::container::{FileItem, FileKind};
pub use crate::stream::CompressionKind;
pub use crate::stream::StreamKind;

fn handle_container<F>(path: &Path, mut archive: impl Container, callback: &mut F)
where
    F: FnMut(FileItem<&mut dyn Read>),
{
    let mut items = archive.items();
    while let Some(mut x) = items.next_item() {
        let reader = &mut x.reader as &mut dyn Read;
        read_recursive_inner(path.join(x.path).as_path(), x.kind, reader, callback);
    }
}

fn read_recursive_inner<F>(path: &Path, kind: FileKind, reader: &mut dyn Read, callback: &mut F)
where
    F: FnMut(FileItem<&mut dyn Read>),
{
    let container = ContainerKind::from_reader(reader);
    match container {
        ContainerKind::Stream(StreamKind::Raw(mut r)) => {
            callback(FileItem {
                path: path.to_path_buf(),
                reader: &mut r as &mut dyn Read,
                kind,
            });
        }
        ContainerKind::Stream(StreamKind::Compressed(mut c)) => {
            read_recursive_inner(path, kind, &mut c as &mut dyn Read, callback)
        }
        ContainerKind::Archive(ArchiveKind::Tar(r)) => handle_container(path, r, callback),
        ContainerKind::Archive(ArchiveKind::Zip(r)) => handle_container(path, r, callback),
    }
}

pub fn read_recursive<F>(path: &Path, mut reader: impl Read, callback: &mut F)
where
    F: FnMut(FileItem<&mut dyn Read>),
{
    read_recursive_inner(path, FileKind::File, &mut reader as &mut dyn Read, callback);
}
