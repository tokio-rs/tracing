use flate2::Compression;

#[derive(Debug, Clone)]
enum GzipCompressionLevelLiteral {
    None,
    Fast,
    Best,
}

#[derive(Debug, Clone)]
enum GzipCompressionLevelNumerical {
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
enum GzipCompressionLevel {
    Literal(GzipCompressionLevelLiteral),
    Numerical(GzipCompressionLevelNumerical),
}

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
struct GzipCompression {
    pub level: GzipCompressionLevel,
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

pub mod compression_options {
    use super::*;
    pub const GZIP_NONE: CompressionConfig = CompressionConfig::Gzip(GzipCompression {
        level: GzipCompressionLevel::Literal(GzipCompressionLevelLiteral::None),
    });
    pub const GZIP_FAST: CompressionConfig = CompressionConfig::Gzip(GzipCompression {
        level: GzipCompressionLevel::Literal(GzipCompressionLevelLiteral::Fast),
    });
    pub const GZIP_BEST: CompressionConfig = CompressionConfig::Gzip(GzipCompression {
        level: GzipCompressionLevel::Literal(GzipCompressionLevelLiteral::Best),
    });
    pub const GZIP_0: CompressionConfig = CompressionConfig::Gzip(GzipCompression {
        level: GzipCompressionLevel::Numerical(GzipCompressionLevelNumerical::Level0),
    });
    pub const GZIP_1: CompressionConfig = CompressionConfig::Gzip(GzipCompression {
        level: GzipCompressionLevel::Numerical(GzipCompressionLevelNumerical::Level1),
    });
    pub const GZIP_2: CompressionConfig = CompressionConfig::Gzip(GzipCompression {
        level: GzipCompressionLevel::Numerical(GzipCompressionLevelNumerical::Level2),
    });
    pub const GZIP_3: CompressionConfig = CompressionConfig::Gzip(GzipCompression {
        level: GzipCompressionLevel::Numerical(GzipCompressionLevelNumerical::Level3),
    });
    pub const GZIP_4: CompressionConfig = CompressionConfig::Gzip(GzipCompression {
        level: GzipCompressionLevel::Numerical(GzipCompressionLevelNumerical::Level4),
    });
    pub const GZIP_5: CompressionConfig = CompressionConfig::Gzip(GzipCompression {
        level: GzipCompressionLevel::Numerical(GzipCompressionLevelNumerical::Level5),
    });
    pub const GZIP_6: CompressionConfig = CompressionConfig::Gzip(GzipCompression {
        level: GzipCompressionLevel::Numerical(GzipCompressionLevelNumerical::Level6),
    });
    pub const GZIP_7: CompressionConfig = CompressionConfig::Gzip(GzipCompression {
        level: GzipCompressionLevel::Numerical(GzipCompressionLevelNumerical::Level7),
    });
    pub const GZIP_8: CompressionConfig = CompressionConfig::Gzip(GzipCompression {
        level: GzipCompressionLevel::Numerical(GzipCompressionLevelNumerical::Level8),
    });
    pub const GZIP_9: CompressionConfig = CompressionConfig::Gzip(GzipCompression {
        level: GzipCompressionLevel::Numerical(GzipCompressionLevelNumerical::Level9),
    });
}
