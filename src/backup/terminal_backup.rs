use std::{io, mem, os::fd::RawFd};

use libc::{tcgetattr, termios};

pub(crate) fn get_terminal_attr(fd: RawFd) -> io::Result<termios> {
    unsafe {
        let mut tios = mem::zeroed();
        wrap_with_result(tcgetattr(fd, &mut tios))?;
        Ok(tios)
    }
}

pub(crate) fn wrap_with_result(result: i32) -> io::Result<()> {
    match result {
        -1 => Err(io::Error::last_os_error()),
        _ => Ok(()),
    }
}
