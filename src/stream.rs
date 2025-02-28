use crate::peekable::{Peekable, PeekableReader};
use flate2::read::GzDecoder;
use std::fmt::{Debug, Formatter};
use std::io::{BufReader, Read};
use tracing::trace;

pub enum StreamKind<T: Read, const N: usize> {
    Compressed(CompressionKind<T, N>),
    Raw(PeekableReader<T, N>),
}

impl<T: Read, const N: usize> Debug for StreamKind<T, N> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamKind::Compressed(c) => write!(f, "StreamKind::Compressed({:?})", c),
            StreamKind::Raw(_) => f.write_str("StreamKind::Raw"),
        }
    }
}

impl<T: Read, const N: usize> Read for StreamKind<T, N> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            StreamKind::Compressed(r) => r.read(buf),
            StreamKind::Raw(r) => r.read(buf),
        }
    }
}

impl<T: Read, const N: usize> StreamKind<T, N> {
    pub fn from_peekable(peekable: Peekable<T, N>) -> StreamKind<T, N> {
        let buf = peekable.peek_buf();

        if infer::archive::is_gz(buf) {
            trace!("gzip detected");
            let decoder = GzDecoder::new(peekable.into_reader());
            StreamKind::Compressed(CompressionKind::Gzip(decoder))
        } else if is_zstd(buf) {
            trace!("zstd detected");
            let decoder =
                zstd::Decoder::with_buffer(BufReader::new(peekable.into_reader())).unwrap();
            StreamKind::Compressed(CompressionKind::Zst(decoder))
        } else if infer::archive::is_bz2(buf) {
            trace!("bzip2 detected");
            let decoder = bzip2::read::BzDecoder::new(peekable.into_reader());
            StreamKind::Compressed(CompressionKind::Bzip2(decoder))
        } else if infer::archive::is_xz(buf) {
            trace!("xz detected");
            let decoder = liblzma::read::XzDecoder::new_multi_decoder(peekable.into_reader());
            StreamKind::Compressed(CompressionKind::Xz(decoder))
        } else {
            trace!("raw detected");
            StreamKind::Raw(peekable.into_reader())
        }
    }
}

// Lower value for compression detection only.
const STREAM_BUF_SIZE: usize = 8;

impl<T: Read> StreamKind<T, STREAM_BUF_SIZE> {
    pub fn from_reader(reader: T) -> StreamKind<T, STREAM_BUF_SIZE> {
        let peekable: Peekable<T, 8> = Peekable::new(reader);
        Self::from_peekable(peekable)
    }
}

pub enum CompressionKind<T: Read, const N: usize> {
    Gzip(GzDecoder<PeekableReader<T, N>>),
    Zst(zstd::Decoder<'static, BufReader<PeekableReader<T, N>>>),
    Bzip2(bzip2::read::BzDecoder<PeekableReader<T, N>>),
    Xz(liblzma::read::XzDecoder<PeekableReader<T, N>>),
}

impl<T: Read, const N: usize> Debug for CompressionKind<T, N> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gzip(_) => f.write_str("Gzip"),
            Self::Zst(_) => f.write_str("Zstd"),
            Self::Bzip2(_) => f.write_str("Bzip2"),
            Self::Xz(_) => f.write_str("Xz"),
        }
    }
}

impl<T: Read, const N: usize> Read for CompressionKind<T, N> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Gzip(r) => r.read(buf),
            Self::Zst(r) => r.read(buf),
            Self::Bzip2(r) => r.read(buf),
            Self::Xz(r) => r.read(buf),
        }
    }
}

fn is_zstd(buffer: &[u8]) -> bool {
    // https://github.com/facebook/zstd/blob/dev/doc/zstd_compression_format.md#zstandard-frames
    // 4 Bytes, little-endian format. Value : 0xFD2FB528

    const SKIPPABLE_FRAME_BASE: u32 = 0x184D2A50;
    const SKIPPABLE_FRAME_MASK: u32 = 0xFFFFFFF0;
    const ZSTD_MAGIC_NUMBER: u32 = 0xFD2FB528;

    if buffer.len() < 4 {
        return false;
    }

    let magic_from_buffer = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
    magic_from_buffer == ZSTD_MAGIC_NUMBER
        || (magic_from_buffer & SKIPPABLE_FRAME_MASK) == SKIPPABLE_FRAME_BASE
}

#[cfg(test)]
mod tests {
    use crate::stream::{CompressionKind, StreamKind};
    use assert_matches::assert_matches;
    use std::io::Write;

    use tracing_test::traced_test;

    const DATA: &[u8] = b"hello world";

    #[traced_test]
    #[test]
    fn test_raw_file() {
        let reader = std::io::Cursor::new(DATA);
        let file_kind = StreamKind::from_reader(reader);
        assert_matches!(file_kind, StreamKind::Raw(_));
    }

    #[traced_test]
    #[test]
    fn test_gzip_file() {
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), Default::default());
        encoder.write_all(DATA).unwrap();
        let compressed_data = encoder.finish().unwrap();
        let file_kind = StreamKind::from_reader(compressed_data.as_slice());
        assert_matches!(file_kind, StreamKind::Compressed(CompressionKind::Gzip(_)));
    }

    #[traced_test]
    #[test]
    fn test_zstd_file() {
        let data = zstd::encode_all(DATA, 1).unwrap();
        let file_kind = StreamKind::from_reader(data.as_slice());
        assert_matches!(file_kind, StreamKind::Compressed(CompressionKind::Zst(_)));
    }

    #[traced_test]
    #[test]
    fn test_bzip2() {
        let mut data = bzip2::write::BzEncoder::new(Vec::new(), Default::default());
        data.write_all(DATA).unwrap();
        let data = data.finish().unwrap();
        let file_kind = StreamKind::from_reader(data.as_slice());
        assert_matches!(file_kind, StreamKind::Compressed(CompressionKind::Bzip2(_)));
    }

    #[traced_test]
    #[test]
    fn test_xz() {
        let mut data = liblzma::write::XzEncoder::new(Vec::new(), 1);
        data.write_all(DATA).unwrap();
        let data = data.finish().unwrap();
        let file_kind = StreamKind::from_reader(data.as_slice());
        assert_matches!(file_kind, StreamKind::Compressed(CompressionKind::Xz(_)));
    }
}
