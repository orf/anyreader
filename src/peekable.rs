use std::io::{Chain, Read};

pub struct Buf<const N: usize> {
    inner: [u8; N],
    len: usize,
}

impl<const N: usize> Buf<N> {
    pub fn new() -> Self {
        Self {
            inner: [0; N],
            len: 0,
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.inner[..self.len]
    }

    pub fn append_from_reader(&mut self, reader: &mut impl Read) -> usize {
        let len = self.len;
        // let remaining = N - len;
        let read = reader.read(&mut self.inner[len..]).unwrap();
        self.len += read;
        read
    }
}

impl<const N: usize> AsRef<[u8]> for Buf<N> {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

pub struct PeekableReader<T, const N: usize> {
    chain: Chain<std::io::Cursor<Buf<N>>, T>,
}

impl<T: Read, const N: usize> Read for PeekableReader<T, N> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.chain.read(buf)
    }
}

impl<T: Read, const N: usize> PeekableReader<T, N> {
    pub fn peek_buf(&self) -> &[u8] {
        let (cursor, _) = self.chain.get_ref();
        cursor.get_ref().as_slice()
    }
}

pub struct Peekable<T: Read, const N: usize> {
    buf: std::io::Cursor<Buf<N>>,
    reader: T,
}

impl<T: Read, const N: usize> Peekable<T, N> {
    pub fn new(mut reader: T) -> Self {
        let mut buf = Buf::new();
        let mut total_read = 0;

        while total_read < N {
            let read = buf.append_from_reader(&mut reader);
            if read == 0 {
                break;
            }
            total_read += read;
        }
        let buf = std::io::Cursor::new(buf);
        Self { buf, reader }
    }

    pub fn into_reader(self) -> PeekableReader<T, N> {
        PeekableReader {
            chain: self.buf.chain(self.reader),
        }
    }

    pub fn peek_buf(&self) -> &[u8] {
        self.buf.get_ref().as_slice()
    }
}
