// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
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
    pub(super) fn try_read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Socket(stream) => {
                stream.set_nonblocking(true)?;
                let result = stream.read(bytes);
                stream.set_nonblocking(false)?;
                result
            }
            #[cfg(test)]
            Self::Memory(memory) => {
                memory.set_read_timeout(Some(Duration::ZERO))?;
                memory.read(bytes)
            }
        }
    }
    pub(super) fn try_write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        match self {
            Self::Socket(stream) => {
                // The connection has one IO owner. Restore its blocking mode
                // before returning to the ordinary synchronous control path.
                stream.set_nonblocking(true)?;
                let result = stream.write(bytes);
                stream.set_nonblocking(false)?;
                result
            }
            #[cfg(test)]
            Self::Memory(memory) => memory.write(bytes),
        }
    }
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
