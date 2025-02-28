use crate::container::{Container, FileItem, FileKind, Items};
use std::fmt::Debug;
use std::io::Read;

pub struct ZipContainer<T: Read> {
    reader: T,
}

impl<T: Read> Debug for ZipContainer<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Zip").finish()
    }
}

impl<T: Read> ZipContainer<T> {
    pub fn new(reader: T) -> Self {
        Self { reader }
    }
}

impl<T: Read> Container for ZipContainer<T> {
    fn items(&mut self) -> impl Items {
        ZipFileIter {
            reader: &mut self.reader,
        }
    }
}

pub struct ZipFileIter<'a, T: Read> {
    reader: &'a mut T,
}

impl<T: Read> Items for ZipFileIter<'_, T> {
    fn next_item(&mut self) -> Option<FileItem<impl Read>> {
        if let Ok(Some(item)) = zip::read::read_zipfile_from_stream(&mut self.reader) {
            let path = item.enclosed_name().unwrap().to_path_buf();
            let kind = if item.is_file() {
                FileKind::File
            } else if item.is_dir() {
                FileKind::Directory
            } else {
                FileKind::Other
            };
            return Some(FileItem {
                path,
                reader: item,
                kind,
            });
        }
        None
    }
}
