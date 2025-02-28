use crate::container::{Container, FileItem, FileKind, Items};
use std::fmt::Debug;
use std::io::Read;
use tar::EntryType;

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
    fn items(&mut self) -> impl Items {
        self.archive.entries().unwrap().map(move |item| {
            let item = item.unwrap();
            let kind = match item.header().entry_type() {
                EntryType::Regular => FileKind::File,
                EntryType::Directory => FileKind::Directory,
                _ => FileKind::Other,
            };
            let path = item.path().unwrap().to_path_buf();
            FileItem {
                path,
                reader: item,
                kind,
            }
        })
    }
}
