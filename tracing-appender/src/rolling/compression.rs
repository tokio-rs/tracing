//! Defines configuration for passing compression options
//!
//! Currently only gzip compression is implemented.
use flate2::Compression;

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum GzipCompressionLevelLiteral {
    None,
    Fast,
    Best,
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[repr(u32)]
pub(crate) enum GzipCompressionLevelNumerical {
    Level0 = 0,
    Level1 = 1,
    Level2 = 2,
    Level3 = 3,
    Level4 = 4,
    Level5 = 5,
    Level6 = 6,
    Level7 = 7,
    Level8 = 8,
    Level9 = 9,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum GzipCompressionLevel {
    Literal(GzipCompressionLevelLiteral),
    Numerical(GzipCompressionLevelNumerical),
}

/// Defines a conversion between `CompressionOption` and `flate2::Compression`
impl Into<Compression> for GzipCompressionLevel {
    fn into(self) -> Compression {
        match self {
            GzipCompressionLevel::Literal(lit) => match lit {
                GzipCompressionLevelLiteral::None => Compression::none(),
                GzipCompressionLevelLiteral::Fast => Compression::fast(),
                GzipCompressionLevelLiteral::Best => Compression::best(),
            },
            GzipCompressionLevel::Numerical(num) => Compression::new(num as u32),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct GzipCompression {
    pub(crate) level: GzipCompressionLevel,
}

/// Data structure to pass compression parameters
#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum CompressionConfig {
    Gzip(GzipCompression),
}

impl CompressionConfig {
    pub(crate) fn gz_compress_level(&self) -> Compression {
        match self {
            CompressionConfig::Gzip(gz) => {
                let level = gz.level.clone().into();
                level
            }
        }
    }

    #[allow(unused)]
    pub(crate) fn extension(&self) -> Option<&str> {
        match self {
            CompressionConfig::Gzip(_) => Some("gz"),
        }
    }
}

/// Defines a compression level for gzip algorithm.
///
/// Compression levels are defined as they are in `flate2` crate where
/// - compression level 0 (`CompressionOption::GzipNone` or `CompressionOption::GzipLevel0`)
/// - compression level 1 (`CompressionOption::GzipFast` or `CompressionOption::GzipLevel1`)
/// - compression level n (where n is between 2 and 9)
/// - compression level 9 (`CompressionOption::GzipBest` or `CompressionOption::GzipLevel9`)
///
/// ```rust
/// # #[cfg(feature = "gzip_compression")] {
/// # fn docs() {
/// use tracing_appender::compression::CompressionOption;
/// let compression_level = CompressionOption::GzipBest;
/// # }
/// # }
/// ```
#[derive(Debug, Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum CompressionOption {
    /// No compression (gzip compression level 0)
    GzipNone,
    /// Fast compression (gzip compression level 1)
    GzipFast,
    /// Fast compression (gzip compression level 9)
    GzipBest,
    /// Gzip compression level 0
    GzipLevel0,
    /// Gzip compression level 1
    GzipLevel1,
    /// Gzip compression level 2
    GzipLevel2,
    /// Gzip compression level 3
    GzipLevel3,
    /// Gzip compression level 4
    GzipLevel4,
    /// Gzip compression level 5
    GzipLevel5,
    /// Gzip compression level 6
    GzipLevel6,
    /// Gzip compression level 7
    GzipLevel7,
    /// Gzip compression level 8
    GzipLevel8,
    /// Gzip compression level 9
    GzipLevel9,
}

impl Into<CompressionConfig> for CompressionOption {
    fn into(self) -> CompressionConfig {
        match self {
            CompressionOption::GzipNone => CompressionConfig::Gzip(GzipCompression {
                level: GzipCompressionLevel::Literal(GzipCompressionLevelLiteral::None),
            }),
            CompressionOption::GzipFast => CompressionConfig::Gzip(GzipCompression {
                level: GzipCompressionLevel::Literal(GzipCompressionLevelLiteral::Fast),
            }),
            CompressionOption::GzipBest => CompressionConfig::Gzip(GzipCompression {
                level: GzipCompressionLevel::Literal(GzipCompressionLevelLiteral::Best),
            }),
            CompressionOption::GzipLevel0 => CompressionConfig::Gzip(GzipCompression {
                level: GzipCompressionLevel::Numerical(GzipCompressionLevelNumerical::Level0),
            }),
            CompressionOption::GzipLevel1 => CompressionConfig::Gzip(GzipCompression {
                level: GzipCompressionLevel::Numerical(GzipCompressionLevelNumerical::Level1),
            }),
            CompressionOption::GzipLevel2 => CompressionConfig::Gzip(GzipCompression {
                level: GzipCompressionLevel::Numerical(GzipCompressionLevelNumerical::Level2),
            }),
            CompressionOption::GzipLevel3 => CompressionConfig::Gzip(GzipCompression {
                level: GzipCompressionLevel::Numerical(GzipCompressionLevelNumerical::Level3),
            }),
            CompressionOption::GzipLevel4 => CompressionConfig::Gzip(GzipCompression {
                level: GzipCompressionLevel::Numerical(GzipCompressionLevelNumerical::Level4),
            }),
            CompressionOption::GzipLevel5 => CompressionConfig::Gzip(GzipCompression {
                level: GzipCompressionLevel::Numerical(GzipCompressionLevelNumerical::Level5),
            }),
            CompressionOption::GzipLevel6 => CompressionConfig::Gzip(GzipCompression {
                level: GzipCompressionLevel::Numerical(GzipCompressionLevelNumerical::Level6),
            }),
            CompressionOption::GzipLevel7 => CompressionConfig::Gzip(GzipCompression {
                level: GzipCompressionLevel::Numerical(GzipCompressionLevelNumerical::Level7),
            }),
            CompressionOption::GzipLevel8 => CompressionConfig::Gzip(GzipCompression {
                level: GzipCompressionLevel::Numerical(GzipCompressionLevelNumerical::Level8),
            }),
            CompressionOption::GzipLevel9 => CompressionConfig::Gzip(GzipCompression {
                level: GzipCompressionLevel::Numerical(GzipCompressionLevelNumerical::Level9),
            }),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::rolling::compression::CompressionOption;
    use crate::rolling::test::write_to_log;
    use crate::rolling::{Builder, Rotation};
    use flate2::read::GzDecoder;
    use std::fs;
    use std::io::Read;
    use std::path::Path;

    fn find_str_in_compressed_log(dir_path: &Path, expected_value: &str) -> bool {
        let dir_contents = fs::read_dir(dir_path).expect("Failed to read directory");

        for entry in dir_contents {
            let path = entry.expect("Expected dir entry").path();
            let bytes = fs::read(&path).expect("Cannot read bytes from compressed log");
            let mut decoder = GzDecoder::new(&bytes[..]);
            let mut s = String::new();
            let r = decoder
                .read_to_string(&mut s)
                .expect("Cannot decode compressed log file");
            if s.as_str() == expected_value {
                return true;
            }
        }

        false
    }

    #[test]
    fn test_compressed_appender() {
        let file_prefix = "my-app-compressed-log";
        let directory = tempfile::tempdir().expect("failed to create tempdir");
        let mut appender = Builder::new(directory.path(), file_prefix)
            .rotation(Rotation::DAILY)
            .compression(CompressionOption::GzipFast)
            .build();

        let expected_value = "Hello";
        write_to_log(&mut appender, expected_value);
        drop(appender);
        assert!(find_str_in_compressed_log(directory.path(), expected_value));

        directory
            .close()
            .expect("Failed to explicitly close TempDir. TempDir should delete once out of scope.")
    }
}
