//! memfd helpers.

use libc::*;
use std::fs::File;
use std::io::Error;
use std::io::Result;
use std::os::raw::c_uint;
use std::os::unix::prelude::{FromRawFd, RawFd};

fn create(flags: c_uint) -> Result<File> {
    let fd = unsafe {
        syscall(
            SYS_memfd_create,
            "tracing-journald\0".as_ptr() as *const c_char,
            flags,
        )
    };
    if fd < 0 {
        Err(Error::last_os_error())
    } else {
        Ok(unsafe { File::from_raw_fd(fd as RawFd) })
    }
}

pub fn create_sealable() -> Result<File> {
    create(MFD_ALLOW_SEALING | MFD_CLOEXEC)
}

pub fn seal_fully(fd: RawFd) -> Result<()> {
    let all_seals = F_SEAL_SHRINK | F_SEAL_GROW | F_SEAL_WRITE | F_SEAL_SEAL;
    let result = unsafe { fcntl(fd, F_ADD_SEALS, all_seals) };
    if result < 0 {
        Err(Error::last_os_error())
    } else {
        Ok(())
    }
}
