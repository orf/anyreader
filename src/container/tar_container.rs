use crate::container::{Container, Items};
use std::fmt::Debug;
use std::io;
use std::io::Read;
use tar::EntryType;
use crate::{FileItem, FileKind};

pub struct TarContainer<T: Read> {
    archive: tar::Archive<T>,
}

impl<T: Read> Debug for TarContainer<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tar").finish()
    }
}

impl<T: Read> TarContainer<T> {
    pub fn new(reader: T) -> Self {
        let archive = tar::Archive::new(reader);
        Self { archive }
    }
}

impl<T: Read> Container for TarContainer<T> {
    fn items(&mut self) -> io::Result<impl Items> {
        Ok(self.archive.entries()?.map(move |item| {
            let item = item?;
            let kind = match item.header().entry_type() {
                EntryType::Regular => FileKind::File,
                EntryType::Directory => FileKind::Directory,
                _ => FileKind::Other,
            };
            let path = item.path()?.to_path_buf();
            Ok(FileItem {
                path,
                reader: item,
                kind,
            })
        }))
    }
}
