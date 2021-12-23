use flate2::Compression;

pub enum GzipCompressionLevelLiteral {
    None,
    Fast,
    Best,
}

pub enum GzipCompressionLevelNumerical {
    Level0,
    Level1,
    Level2,
    Level3,
    Level4,
    Level5,
    Level6,
    Level7,
    Level8,
    Level9
}

pub enum GzipCompressionLevel {
    Literal(GzipCompressionLevelLiteral),
    Numerical(GzipCompressionLevelNumerical)
}

impl Into<Compression> for GzipCompressionLevel {
    fn into(self) -> Compression {
        match GzipCompressionLevel {
            GzipCompressionLevel::Literal(lit) => {
                match lit {
                    GzipCompressionLevelLiteral::None => Compression::none(),
                    GzipCompressionLevelLiteral::Fast => Compression::fast(),
                    GzipCompressionLevelLiteral::Best => Compression::best()
                }
            },
            GzipCompressionLevel::Numerical(num) => {
                match num {
                    GzipCompressionLevelNumerical::Level0 => Compression(0),
                    GzipCompressionLevelNumerical::Level1 => Compression(1),
                    GzipCompressionLevelNumerical::Level2 => Compression(2),
                    GzipCompressionLevelNumerical::Level3 => Compression(3),
                    GzipCompressionLevelNumerical::Level4 => Compression(4),
                    GzipCompressionLevelNumerical::Level5 => Compression(5),
                    GzipCompressionLevelNumerical::Level6 => Compression(6),
                    GzipCompressionLevelNumerical::Level7 => Compression(7),
                    GzipCompressionLevelNumerical::Level8 => Compression(8),
                    GzipCompressionLevelNumerical::Level9 => Compression(9)
                }
            }
        }
    }
}

pub struct GzipCompression {
    pub level: GzipCompressionLevel
}

/// Data structure to pass compression parameters
pub enum CompressionConfig {
    Gzip(GzipCompression)
}