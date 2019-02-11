use std::{
    fmt, io::{self, ErrorKind}, str,
    sync::atomic::{AtomicUsize, Ordering},
};

#[derive(Debug)]
pub struct Data {
    pub(crate) name: &'static str,
    pub(crate) fields: String,
    pub(crate) ref_count: AtomicUsize,
}


// ===== impl Data =====

impl Data {
    pub(crate) fn new(name: &'static str) -> Self {
        Self {
            name,
            fields: String::new(),
            ref_count: AtomicUsize::new(1),
        }
    }

    #[inline]
    pub(crate) fn clone_ref(&self) {
        self.ref_count.fetch_add(1, Ordering::Release);
    }

    #[inline]
    pub(crate) fn drop_ref(&self) -> bool {
        self.ref_count.fetch_sub(1, Ordering::AcqRel) == 1
    }
}

impl io::Write for Data {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Hopefully consumers of this struct will only use the `write_fmt`
        // impl, which should be much faster.
        let string = str::from_utf8(buf)
            .map_err(|e| io::Error::new(ErrorKind::Other, e))?;
        self.fields.push_str(string);
        Ok(buf.len())
    }

    #[inline]
    fn write_fmt(&mut self, args: fmt::Arguments) -> io::Result<()> {
        use fmt::Write;
        self.fields.write_fmt(args)
            .map_err(|e| io::Error::new(ErrorKind::Other, e))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
