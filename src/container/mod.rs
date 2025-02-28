mod tar_container;
mod zip_container;

use crate::container::tar_container::TarContainer;
use crate::container::zip_container::ZipContainer;
use crate::peekable::Peekable;
use crate::stream::StreamKind;
use std::fmt::{Debug, Formatter};
use std::io;
use std::io::Read;
use tracing::trace;
use crate::FileItem;

// Annoying: this needs to be quite high to detect tar archives
const ARCHIVE_BUF_SIZE: usize = 262;

pub enum ArchiveKind<T: Read, const N: usize> {
    Tar(TarContainer<StreamKind<T, N>>),
    Zip(ZipContainer<StreamKind<T, N>>),
}

impl<T: Read, const N: usize> Debug for ArchiveKind<T, N> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tar(s) => {
                write!(f, "ArchiveKind::Tar({s:?})")
            }
            ArchiveKind::Zip(s) => {
                write!(f, "ArchiveKind::Zip({s:?})")
            }
        }
    }
}

pub enum ContainerKind<T: Read, const N: usize = ARCHIVE_BUF_SIZE> {
    Stream(StreamKind<T, N>),
    Archive(ArchiveKind<T, N>),
}

impl<T: Read, const N: usize> Debug for ContainerKind<T, N> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stream(s) => write!(f, "ContainerKind::Stream({s:?})"),
            Self::Archive(s) => write!(f, "ContainerKind::Archive({s:?})"),
        }
    }
}

impl<T: Read> ContainerKind<T, ARCHIVE_BUF_SIZE> {
    pub fn from_reader(reader: T) -> io::Result<ContainerKind<T, ARCHIVE_BUF_SIZE>> {
        let peekable = Peekable::new(reader)?;
        let kind = StreamKind::from_peekable(peekable)?;
        match kind {
            StreamKind::Compressed(c) => Ok(Self::Stream(StreamKind::Compressed(c))),
            StreamKind::Raw(r) => {
                let buf = r.peek_buf();
                if infer::archive::is_tar(buf) {
                    trace!("tar detected");
                    Ok(ContainerKind::Archive(ArchiveKind::Tar(TarContainer::new(StreamKind::Raw(r)))))
                } else if infer::archive::is_zip(buf) {
                    trace!("zip detected");
                    Ok(ContainerKind::Archive(ArchiveKind::Zip(ZipContainer::new(StreamKind::Raw(r)))))
                } else {
                    trace!("stream detected");
                    Ok(ContainerKind::Stream(StreamKind::Raw(r)))
                }
            }
        }
    }
}

pub trait Container {
    fn items(&mut self) -> io::Result<impl Items>;
}

pub trait Items {
    fn next_item(&mut self) -> Option<io::Result<FileItem<impl Read>>>;
}

impl<T, R> Items for T
where
    R: Read,
    T: Iterator<Item = io::Result<FileItem<R>>>,
{
    fn next_item(&mut self) -> Option<io::Result<FileItem<impl Read>>> {
        self.next()
    }
}

#[cfg(test)]
mod tests {
    use crate::container::ContainerKind;
    use crate::stream::{CompressionKind, StreamKind};
    use assert_matches::assert_matches;
    use std::io::Write;

    use tracing_test::traced_test;

    const DATA: &[u8] = b"hello world";

    #[traced_test]
    #[test]
    fn test_recursive_compression() {
        let zstd_data = zstd::encode_all(DATA, 1).unwrap();
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), Default::default());
        encoder.write_all(&zstd_data).unwrap();
        let compressed_data = encoder.finish().unwrap();
        let gzip_file_kind = ContainerKind::from_reader(compressed_data.as_slice()).unwrap();
        let stream_kind = assert_matches!(
            gzip_file_kind,
            ContainerKind::Stream(StreamKind::Compressed(CompressionKind::Gzip(r))) => r
        );
        let stream_kind = ContainerKind::from_reader(stream_kind).unwrap();
        assert_matches!(
            stream_kind,
            ContainerKind::Stream(StreamKind::Compressed(CompressionKind::Zst(_)))
        );
    }
}
