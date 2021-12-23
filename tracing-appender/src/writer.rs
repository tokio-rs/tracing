use std::{fs, io};
use std::borrow::BorrowMut;
use std::io::{BufWriter, Write};
use std::fs::{File, OpenOptions};
use std::ops::{Deref, DerefMut};
use std::path::Path;
use crate::sync::RwLock;
use crate::rolling::create_writer_file;
#[cfg(feature = "compression")]
use flate2::write::GzEncoder;

#[derive(Debug)]
pub enum WriterChannel {
    File(File),
    #[cfg(feature = "compression")]
    CompressedFileGzip(BufWriter<GzEncoder<BufWriter<File>>>),
}

impl WriterChannel {
    #[cfg(feature = "compression")]
    pub fn new(directory: &str,
               filename: &str,
               #[cfg(feature = "compression")]
               compression: CompressionConfig) -> io::Result<Self> {

        if let Some(compression) = compression {
            Self::new_with_compression(directory, filename, compression)
        } else {
            Self::new_without_compression(directory, filename)
        }
    }

    #[cfg(not(feature = "compression"))]
    pub fn new(directory: &str,
               filename: &str) -> io::Result<Self> {
        Self::new_without_compression(directory, filename)
    }

    pub fn new_without_compression(directory: &str, filename: &str) -> io::Result<Self> {
        let file = create_writer_file(directory, filename)?;
        Ok(WriterChannel::File(file))
    }

    #[cfg(feature = "compression")]
    pub fn new_with_compression(directory: &str,
                                filename: &str,
                                compression: CompressionConfig) -> io::Result<Self> {
        let file = create_writer_file(directory, filename)?;
        let buf = BufWriter::new(file);
        let gzfile = GzEncoder::new(buf, compression.into());
        let writer = BufWriter::new(gzfile);
        Ok(WriterChannel::CompressedFileGzip(writer))
    }

    pub fn get_writer(&mut self) -> &mut dyn Write {
        match self {
            WriterChannel::File(x) => x,
            #[cfg(feature = "compression")]
            WriterChannel::CompressedFileGzip(x) => x,
        }
    }
}

impl io::Write for WriterChannel {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.borrow_mut().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.borrow_mut().flush()
    }
}
