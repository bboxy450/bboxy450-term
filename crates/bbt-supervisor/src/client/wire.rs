//! Native IO in production; the contract tests substitute bytes and a clock.
use std::io::{self, Read, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

pub(super) enum Wire {
    Socket(UnixStream),
    #[cfg(test)]
    Memory(super::tests::Memory),
}

impl From<UnixStream> for Wire {
    fn from(stream: UnixStream) -> Self {
        Self::Socket(stream)
    }
}

impl Wire {
    pub(super) fn now(&self) -> Instant {
        match self {
            Self::Socket(_) => Instant::now(),
            #[cfg(test)]
            Self::Memory(memory) => memory.now(),
        }
    }
    pub(super) fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        match self {
            Self::Socket(stream) => stream.set_read_timeout(timeout),
            #[cfg(test)]
            Self::Memory(memory) => memory.set_read_timeout(timeout),
        }
    }
    pub(super) fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        match self {
            Self::Socket(stream) => stream.shutdown(how),
            #[cfg(test)]
            Self::Memory(memory) => memory.shutdown(),
        }
    }
}

impl Read for Wire {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Socket(stream) => stream.read(bytes),
            #[cfg(test)]
            Self::Memory(memory) => memory.read(bytes),
        }
    }
}
impl Write for Wire {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        match self {
            Self::Socket(stream) => stream.write(bytes),
            #[cfg(test)]
            Self::Memory(memory) => memory.write(bytes),
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Socket(stream) => stream.flush(),
            #[cfg(test)]
            Self::Memory(memory) => memory.flush(),
        }
    }
}
