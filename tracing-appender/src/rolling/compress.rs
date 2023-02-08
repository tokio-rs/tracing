use std::{
    io,
    path::{Path, PathBuf},
};

#[cfg(feature = "brotli")]
use brotli::enc::backward_references::BrotliEncoderParams;
#[cfg(feature = "gzip")]
use flate2::Compression as GzCompression;

/// Compression represents howto compress the log files
#[derive(Clone, Debug)]
pub enum Compression {
    /// don't compress
    None,
    /// compress log files as `*.br`
    #[cfg(feature = "brotli")]
    Brotli {
        /// the chunk size to process, defaults to `4096`
        buffer_size: usize,
        /// the extra param to customize, most notably `quality` and `lgwin`
        params: BrotliEncoderParams,
    },
    /// compress log files as `*.gz`
    #[cfg(feature = "gzip")]
    Gzip(GzCompression),
}

impl Default for Compression {
    fn default() -> Self {
        cfg_if::cfg_if! {
            if #[cfg(feature = "gzip")] {
                Self::Gzip(GzCompression::default())
            } else if #[cfg(feature = "brotli")] {
                Self::Brotli {
                    buffer_size: 4096,
                    params: BrotliEncoderParams::default(),
                }
            } else {
                Self::None
            }
        }
    }
}

impl Compression {
    /// construct no compression
    pub const fn none() -> Self {
        Self::None
    }

    /// construct `brotli` compression
    #[cfg(feature = "brotli")]
    pub const fn brotli(buffer_size: usize, params: BrotliEncoderParams) -> Self {
        Self::Brotli {
            buffer_size,
            params,
        }
    }

    /// construct `gzip` compression
    #[cfg(feature = "gzip")]
    pub const fn gzip(level: u32) -> Self {
        Self::Gzip(GzCompression::new(level))
    }

    /// the compressed file extension
    pub(super) fn extension(&self) -> Option<&'static str> {
        match self {
            Self::None => None,
            #[cfg(feature = "brotli")]
            Self::Brotli { .. } => Some("br"),
            #[cfg(feature = "gzip")]
            Self::Gzip(_) => Some("gz"),
        }
    }

    /// do compress
    pub(super) fn compress(&self, #[allow(unused)] path: &Path) -> io::Result<()> {
        match self {
            Compression::None => Ok(()),
            #[cfg(feature = "brotli")]
            Compression::Brotli {
                buffer_size,
                params,
            } => compress_brotli(path, *buffer_size, params),
            #[cfg(feature = "gzip")]
            Compression::Gzip(gz) => compress_gzip(path, *gz),
        }
    }
}

#[allow(unused)]
fn add_extension(path: &Path, ext: &'static str) -> PathBuf {
    let mut path = std::ffi::OsString::from(path);
    path.push(".");
    path.push(ext);
    path.into()
}

#[cfg(feature = "brotli")]
fn compress_brotli(
    path: &Path,
    buffer_size: usize,
    params: &BrotliEncoderParams,
) -> io::Result<()> {
    use std::{
        fs::File,
        io::{BufReader, BufWriter},
    };

    use brotli::enc::writer::CompressorWriter;

    let reader = File::open(path)?;
    let mut reader = BufReader::new(reader);

    let writer = File::create(add_extension(path, "br").as_path())?;
    let writer = CompressorWriter::with_params(writer, buffer_size, params);
    let mut writer = BufWriter::new(writer);

    io::copy(&mut reader, &mut writer)?;
    Ok(())
}

#[cfg(feature = "gzip")]
fn compress_gzip(path: &Path, gz: GzCompression) -> io::Result<()> {
    use std::{
        fs::File,
        io::{BufReader, BufWriter},
    };

    use flate2::write::GzEncoder;

    let reader = File::open(path)?;
    let mut reader = BufReader::new(reader);

    let writer = File::create(add_extension(path, "gz").as_path())?;
    let writer = GzEncoder::new(writer, gz);
    let mut writer = BufWriter::new(writer);

    io::copy(&mut reader, &mut writer)?;
    Ok(())
}
