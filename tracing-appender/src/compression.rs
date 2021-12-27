use flate2::Compression;

#[derive(Debug, Clone)]
pub(crate) enum GzipCompressionLevelLiteral {
    None,
    Fast,
    Best,
}

#[derive(Debug, Clone)]
pub(crate) enum GzipCompressionLevelNumerical {
    Level0,
    Level1,
    Level2,
    Level3,
    Level4,
    Level5,
    Level6,
    Level7,
    Level8,
    Level9,
}

#[derive(Debug, Clone)]
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
            GzipCompressionLevel::Numerical(num) => match num {
                GzipCompressionLevelNumerical::Level0 => Compression::new(0),
                GzipCompressionLevelNumerical::Level1 => Compression::new(1),
                GzipCompressionLevelNumerical::Level2 => Compression::new(2),
                GzipCompressionLevelNumerical::Level3 => Compression::new(3),
                GzipCompressionLevelNumerical::Level4 => Compression::new(4),
                GzipCompressionLevelNumerical::Level5 => Compression::new(5),
                GzipCompressionLevelNumerical::Level6 => Compression::new(6),
                GzipCompressionLevelNumerical::Level7 => Compression::new(7),
                GzipCompressionLevelNumerical::Level8 => Compression::new(8),
                GzipCompressionLevelNumerical::Level9 => Compression::new(9),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct GzipCompression {
    pub(crate) level: GzipCompressionLevel,
}

/// Data structure to pass compression parameters
#[derive(Debug, Clone)]
#[non_exhaustive]
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
/// # fn docs() {
/// use tracing_appender::compression::CompressionOption;
/// let compression_level = CompressionOption::GzipBest;
/// # }
/// ```
#[derive(Debug, Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum CompressionOption {
    GzipNone,
    GzipFast,
    GzipBest,
    GzipLevel0,
    GzipLevel1,
    GzipLevel2,
    GzipLevel3,
    GzipLevel4,
    GzipLevel5,
    GzipLevel6,
    GzipLevel7,
    GzipLevel8,
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
