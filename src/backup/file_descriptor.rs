use std::{fs, io, os::fd::{IntoRawFd, RawFd}};

#[derive(Debug)]
pub(crate) struct FileDesc<'a> {
    fd: RawFd,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl FileDesc<'_> {
    pub(crate) fn raw_fd(&self) -> RawFd {
        self.fd
    }
}

pub(crate) fn create_tty_fd() -> io::Result<FileDesc<'static>> {
    let fd = if unsafe { libc::isatty(libc::STDIN_FILENO) == 1 } {
        libc::STDIN_FILENO
    } else {
        fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/tty")?
            .into_raw_fd()
    };

    let fd = FileDesc {
        fd,
        _phantom: std::marker::PhantomData,
    };

    Ok(fd)
}
