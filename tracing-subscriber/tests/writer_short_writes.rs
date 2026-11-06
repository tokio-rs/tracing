#![cfg(feature = "fmt")]

use std::io::{self, IoSlice, Write};
use tracing_subscriber::fmt::writer::Tee;

struct ShortWriter<'a> {
    output: &'a mut Vec<u8>,
    limit: usize,
}

impl Write for ShortWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = buf.len().min(self.limit);
        self.output.extend_from_slice(&buf[..len]);
        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::ErrorKind::BrokenPipe.into())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn writes_report_only_bytes_written_to_both_outputs() {
    for (limit_a, limit_b) in [(2, 6), (6, 2)] {
        let mut a = Vec::new();
        let mut b = Vec::new();
        let mut tee = Tee::new(
            ShortWriter {
                output: &mut a,
                limit: limit_a,
            },
            ShortWriter {
                output: &mut b,
                limit: limit_b,
            },
        );

        let input = b"abcdef";
        let written = tee.write(input).unwrap();
        assert!(written > 0);
        assert_eq!(a, input[..written]);
        assert_eq!(b, input[..written]);
    }
}

#[test]
fn vectored_writes_report_only_bytes_written_to_both_outputs() {
    for (limit_a, limit_b) in [(2, 6), (6, 2)] {
        let mut a = Vec::new();
        let mut b = Vec::new();
        let mut tee = Tee::new(
            ShortWriter {
                output: &mut a,
                limit: limit_a,
            },
            ShortWriter {
                output: &mut b,
                limit: limit_b,
            },
        );

        let bufs = [
            IoSlice::new(b""),
            IoSlice::new(b"abcd"),
            IoSlice::new(b"ef"),
        ];
        let written = tee.write_vectored(&bufs).unwrap();
        assert!(written > 0);
        assert_eq!(a, b"abcdef"[..written]);
        assert_eq!(b, b"abcdef"[..written]);
    }
}

#[test]
fn writes_attempt_both_outputs_on_error() {
    for fail_first in [true, false] {
        for vectored in [true, false] {
            let mut output = Vec::new();
            let mut failing = FailingWriter;
            let mut tee: Box<dyn Write + '_> = if fail_first {
                Box::new(Tee::new(&mut failing, &mut output))
            } else {
                Box::new(Tee::new(&mut output, &mut failing))
            };

            let error = if vectored {
                tee.write_vectored(&[IoSlice::new(b""), IoSlice::new(b"abcdef")])
                    .unwrap_err()
            } else {
                tee.write(b"abcdef").unwrap_err()
            };
            assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
            drop(tee);
            assert_eq!(output, b"abcdef");
        }
    }
}

#[test]
fn zero_progress_is_an_error_but_other_output_is_written() {
    let mut a = Vec::new();
    let mut b = Vec::new();
    let mut tee = Tee::new(
        ShortWriter {
            output: &mut a,
            limit: 0,
        },
        &mut b,
    );

    let error = tee.write(b"abcdef").unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::WriteZero);
    assert!(a.is_empty());
    assert_eq!(b, b"abcdef");
}

#[test]
fn empty_writes_succeed() {
    let mut tee = Tee::new(FailingWriter, FailingWriter);
    assert_eq!(tee.write(b"").unwrap(), 0);
    assert_eq!(tee.write_vectored(&[]).unwrap(), 0);
    assert_eq!(tee.write_vectored(&[IoSlice::new(b"")]).unwrap(), 0);
}
