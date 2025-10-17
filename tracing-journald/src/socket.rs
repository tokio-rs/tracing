//! socket helpers.

use std::io::{Error, Result};
use std::mem::{size_of, zeroed};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::net::UnixDatagram;
use std::os::unix::prelude::{AsRawFd, RawFd};
use std::path::Path;
use std::ptr;

use libc::*;

const CMSG_BUFSIZE: usize = 64;

#[repr(C)]
union AlignedBuffer<T: Copy + Clone> {
    buffer: T,
    align: cmsghdr,
}

fn assert_cmsg_bufsize() {
    let space_one_fd = unsafe { CMSG_SPACE(size_of::<RawFd>() as u32) };
    assert!(
        space_one_fd <= CMSG_BUFSIZE as u32,
        "cmsghdr buffer too small (< {}) to hold a single fd",
        space_one_fd
    );
}

#[cfg(test)]
#[test]
fn cmsg_buffer_size_for_one_fd() {
    assert_cmsg_bufsize()
}

pub(crate) fn send_one_fd_to<P: AsRef<Path>>(
    socket: &UnixDatagram,
    fd: RawFd,
    path: P,
) -> Result<usize> {
    assert_cmsg_bufsize();

    let mut addr: sockaddr_un = unsafe { zeroed() };
    let path_bytes = path.as_ref().as_os_str().as_bytes();
    // path_bytes may have at most sun_path + 1 bytes, to account for the trailing NUL byte.
    if addr.sun_path.len() <= path_bytes.len() {
        return Err(Error::from_raw_os_error(ENAMETOOLONG));
    }

    addr.sun_family = AF_UNIX as _;
    unsafe {
        std::ptr::copy_nonoverlapping(
            path_bytes.as_ptr(),
            addr.sun_path.as_mut_ptr() as *mut u8,
            path_bytes.len(),
        )
    };

    let mut msg: msghdr = unsafe { zeroed() };
    // Set the target address.
    msg.msg_name = &mut addr as *mut _ as *mut c_void;
    msg.msg_namelen = size_of::<sockaddr_un>() as socklen_t;

    // We send no data body with this message.
    msg.msg_iov = ptr::null_mut();
    msg.msg_iovlen = 0;

    // Create and fill the control message buffer with our file descriptor
    let mut cmsg_buffer = AlignedBuffer {
        buffer: ([0u8; CMSG_BUFSIZE]),
    };
    msg.msg_control = unsafe { cmsg_buffer.buffer.as_mut_ptr() as _ };
    msg.msg_controllen = unsafe { CMSG_SPACE(size_of::<RawFd>() as _) as _ };

    let cmsg: &mut cmsghdr =
        unsafe { CMSG_FIRSTHDR(&msg).as_mut() }.expect("Control message buffer exhausted");

    cmsg.cmsg_level = SOL_SOCKET;
    cmsg.cmsg_type = SCM_RIGHTS;
    cmsg.cmsg_len = unsafe { CMSG_LEN(size_of::<RawFd>() as _) as _ };

    unsafe { ptr::write(CMSG_DATA(cmsg) as *mut RawFd, fd) };

    let result = unsafe { sendmsg(socket.as_raw_fd(), &msg, libc::MSG_NOSIGNAL) };

    if result < 0 {
        Err(Error::last_os_error())
    } else {
        // sendmsg returns the number of bytes written
        Ok(result as usize)
    }
}
