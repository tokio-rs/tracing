use crate::rolling::create_writer_file;
use std::fs::File;
use std::io;

#[cfg(feature = "compression")]
use {
    crate::compression::CompressionConfig,
    flate2::write::GzEncoder,
    std::io::BufWriter,
    std::sync::{Arc, RwLock},
};

#[cfg(feature = "compression")]
#[derive(Debug)]
pub(crate) struct CompressedGzip {
    compression: CompressionConfig,
    buffer: Arc<RwLock<BufWriter<GzEncoder<BufWriter<File>>>>>,
}

#[derive(Debug)]
pub(crate) enum WriterChannel {
    File(File),
    #[cfg(feature = "compression")]
    CompressedFileGzip(CompressedGzip),
}

impl WriterChannel {
    #[cfg(feature = "compression")]
    pub(crate) fn new(
        directory: &str,
        filename: &str,
        #[cfg(feature = "compression")] compression: Option<CompressionConfig>,
    ) -> io::Result<Self> {
        if let Some(compression) = compression {
            Self::new_with_compression(directory, filename, compression)
        } else {
            Self::new_without_compression(directory, filename)
        }
    }

    #[cfg(not(feature = "compression"))]
    pub(crate) fn new(directory: &str, filename: &str) -> io::Result<Self> {
        Self::new_without_compression(directory, filename)
    }

    pub(crate) fn new_without_compression(directory: &str, filename: &str) -> io::Result<Self> {
        let file = create_writer_file(directory, filename)?;
        Ok(WriterChannel::File(file))
    }

    #[cfg(feature = "compression")]
    pub(crate) fn new_with_compression(
        directory: &str,
        filename: &str,
        compression: CompressionConfig,
    ) -> io::Result<Self> {
        let file = create_writer_file(directory, filename)?;
        let buf = BufWriter::new(file);
        let gzfile = GzEncoder::new(buf, compression.gz_compress_level());
        let writer = BufWriter::new(gzfile);
        let compressed_gz = CompressedGzip {
            compression: compression.clone(),
            buffer: Arc::new(RwLock::new(writer)),
        };
        Ok(WriterChannel::CompressedFileGzip(compressed_gz))
    }
}

impl io::Write for WriterChannel {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            WriterChannel::File(f) => f.write(buf),
            #[cfg(feature = "compression")]
            WriterChannel::CompressedFileGzip(gz) => {
                let mut buffer = gz.buffer.write().unwrap();
                buffer.write(buf)
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            WriterChannel::File(f) => f.flush(),
            #[cfg(feature = "compression")]
            WriterChannel::CompressedFileGzip(gz) => {
                let mut buffer = gz.buffer.write().unwrap();
                buffer.flush()
            }
        }
    }
}

impl io::Write for &WriterChannel {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            WriterChannel::File(f) => (&*f).write(buf),
            #[cfg(feature = "compression")]
            WriterChannel::CompressedFileGzip(gz) => {
                let mut buffer = gz.buffer.write().unwrap();
                buffer.write(buf)
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            WriterChannel::File(f) => (&*f).flush(),
            #[cfg(feature = "compression")]
            WriterChannel::CompressedFileGzip(gz) => {
                let mut buffer = gz.buffer.write().unwrap();
                buffer.flush()
            }
        }
    }
}
