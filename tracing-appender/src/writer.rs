use std::io::{BufWriter, Write};
use std::fs::File;
use flate2::write::GzEncoder;

#[derive(Debug)]
pub enum WriterChannel {
    File(BufWriter<File>),
    CompressedFileGzip(BufWriter<GzEncoder<BufWriter<File>>>),
}

impl WriterChannel {
    pub fn get_writer(&mut self) -> &mut dyn Write {
        match self {
            WriterChannel::File(x) => x,
            WriterChannel::CompressedFileGzip(x) => x,
        }
    }
}
