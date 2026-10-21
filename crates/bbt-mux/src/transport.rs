//! Bounded, versioned JSON frames: u32 big-endian length, then UTF-8 JSON.
//!
//! These are private supervisor control frames, not the later public seat bus.

use std::collections::VecDeque;
use std::fs::{self, DirBuilder};
use std::io::{self, Read, Write};
use std::marker::PhantomData;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};

use mio::net::{UnixListener as MioListener, UnixStream};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::protocol::{Frame, Message, Request, VERSION};

pub const MAX_FRAME_BYTES: usize = 1024 * 1024;
pub(crate) const MAX_SOCKET_QUEUE_BYTES: usize = 4 * MAX_FRAME_BYTES;

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

pub fn encode<T: Serialize>(frame: &Frame<T>) -> io::Result<Vec<u8>> {
    if frame.version != VERSION {
        return Err(invalid("unsupported supervisor protocol version"));
    }
    let body = serde_json::to_vec(frame).map_err(invalid)?;
    if body.is_empty() || body.len() > MAX_FRAME_BYTES {
        return Err(invalid("supervisor frame exceeds 1 MiB"));
    }
    let mut bytes = Vec::with_capacity(4 + body.len());
    bytes.extend_from_slice(&(body.len() as u32).to_be_bytes());
    bytes.extend_from_slice(&body);
    Ok(bytes)
}

fn decode_body<T: DeserializeOwned>(body: &[u8]) -> io::Result<Frame<T>> {
    let frame: Frame<T> = serde_json::from_slice(body).map_err(invalid)?;
    if frame.version != VERSION {
        return Err(invalid("unsupported supervisor protocol version"));
    }
    Ok(frame)
}

fn checked_length(bytes: [u8; 4]) -> io::Result<usize> {
    let len = u32::from_be_bytes(bytes) as usize;
    if len == 0 || len > MAX_FRAME_BYTES {
        return Err(invalid("invalid supervisor frame length"));
    }
    Ok(len)
}

/// Blocking helper for a future split-process client. Length checked before allocation.
pub fn read_frame<T: DeserializeOwned>(reader: &mut impl Read) -> io::Result<Frame<T>> {
    let mut header = [0; 4];
    reader.read_exact(&mut header)?;
    let len = checked_length(header)?;
    let mut body = vec![0; len];
    reader.read_exact(&mut body)?;
    decode_body(&body)
}

pub fn write_frame<T: Serialize>(writer: &mut impl Write, frame: &Frame<T>) -> io::Result<()> {
    writer.write_all(&encode(frame)?)
}

/// Incremental decoder used by the nonblocking socket in the supervisor's poller.
pub struct Decoder<T> {
    buffer: Vec<u8>,
    body_len: Option<usize>,
    marker: PhantomData<T>,
}

impl<T> Default for Decoder<T> {
    fn default() -> Self {
        Self { buffer: Vec::new(), body_len: None, marker: PhantomData }
    }
}

impl<T: DeserializeOwned> Decoder<T> {
    pub fn push(&mut self, mut bytes: &[u8]) -> io::Result<Vec<Frame<T>>> {
        let mut frames = Vec::new();
        while !bytes.is_empty() {
            let needed = self.body_len.unwrap_or(4) - self.buffer.len();
            let count = needed.min(bytes.len());
            self.buffer.extend_from_slice(&bytes[..count]);
            bytes = &bytes[count..];
            if self.buffer.len() != self.body_len.unwrap_or(4) {
                continue;
            }
            if self.body_len.is_none() {
                // Reject a forged length before reserving its payload.
                let len = checked_length(self.buffer[..4].try_into().unwrap())?;
                self.buffer.clear();
                self.body_len = Some(len);
            } else {
                frames.push(decode_body(&self.buffer)?);
                self.buffer.clear();
                self.body_len = None;
            }
        }
        Ok(frames)
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty() && self.body_len.is_none()
    }
}

/// Private per-user control endpoint. Nothing binds here merely by using the crate.
pub fn default_socket_path() -> io::Result<PathBuf> {
    #[cfg(target_os = "macos")]
    let base = std::env::var_os("HOME").map(|home| {
        PathBuf::from(home).join("Library/Application Support/bboxy450-term/run")
    });
    #[cfg(not(target_os = "macos"))]
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(|dir| PathBuf::from(dir).join("bboxy450-term"))
        .or_else(|| std::env::var_os("HOME").map(|home| {
            PathBuf::from(home).join(".local/state/bboxy450-term/run")
        }));
    base.map(|dir| dir.join("mux.sock"))
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no private runtime directory"))
}

pub(crate) struct BoundSocket {
    pub listener: MioListener,
    path: PathBuf,
    dev: u64,
    ino: u64,
}

impl BoundSocket {
    pub fn bind(path: &Path) -> io::Result<Self> {
        let parent = path.parent().filter(|p| !p.as_os_str().is_empty())
            .ok_or_else(|| invalid("socket path must have a private parent directory"))?;
        DirBuilder::new().recursive(true).mode(0o700).create(parent)?;
        let metadata = fs::symlink_metadata(parent)?;
        // SAFETY: geteuid has no arguments and no memory-safety preconditions.
        let uid = unsafe { libc::geteuid() };
        if !metadata.is_dir() || metadata.uid() != uid || metadata.mode() & 0o077 != 0 {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "runtime parent must be owned and mode 0700"));
        }
        // Never unlink an existing socket; another supervisor may own it.
        let listener = UnixListener::bind(path)?;
        let metadata = fs::symlink_metadata(path)?;
        let bound = Self {
            listener: MioListener::from_std(listener), path: path.to_owned(),
            dev: metadata.dev(), ino: metadata.ino(),
        };
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
        // mio requires nonblocking descriptors, including those passed from_std.
        crate::supervisor::set_nonblocking(std::os::fd::AsRawFd::as_raw_fd(&bound.listener))?;
        Ok(bound)
    }
}

impl Drop for BoundSocket {
    fn drop(&mut self) {
        if let Ok(metadata) = fs::symlink_metadata(&self.path)
            && metadata.dev() == self.dev && metadata.ino() == self.ino
        {
            let _ = fs::remove_file(&self.path);
        }
    }
}

pub(crate) struct SocketPeer {
    pub stream: UnixStream,
    pub decoder: Decoder<Request>,
    queue: VecDeque<Vec<u8>>,
    offset: usize,
    queued_bytes: usize,
}

impl SocketPeer {
    pub fn new(stream: UnixStream) -> Self {
        Self { stream, decoder: Decoder::default(), queue: VecDeque::new(), offset: 0, queued_bytes: 0 }
    }

    pub fn enqueue(&mut self, message: Message) -> io::Result<bool> {
        let bytes = encode(&Frame::new(message))?;
        if self.queued_bytes + bytes.len() > MAX_SOCKET_QUEUE_BYTES {
            return Ok(false);
        }
        self.queued_bytes += bytes.len();
        self.queue.push_back(bytes);
        Ok(true)
    }

    pub fn wants_write(&self) -> bool {
        !self.queue.is_empty()
    }

    pub fn flush(&mut self) -> io::Result<()> {
        while let Some(front) = self.queue.front() {
            match self.stream.write(&front[self.offset..]) {
                Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
                Ok(count) => {
                    self.offset += count;
                    self.queued_bytes -= count;
                    if self.offset == front.len() {
                        self.queue.pop_front();
                        self.offset = 0;
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(()),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Action;

    #[test]
    fn fragmented_and_coalesced_frames_round_trip() {
        let frame = Frame::new(Request::new(None, Action::ListOwnedSeats));
        let bytes = encode(&frame).unwrap();
        let mut decoder = Decoder::<Request>::default();
        let mut result = Vec::new();
        for byte in &bytes {
            result.extend(decoder.push(&[*byte]).unwrap());
        }
        assert_eq!(result, vec![frame.clone()]);
        assert!(decoder.is_empty());
        let joined = [bytes.as_slice(), bytes.as_slice()].concat();
        assert_eq!(decoder.push(&joined).unwrap(), vec![frame.clone(), frame]);
    }

    #[test]
    fn rejects_lengths_versions_and_truncated_payloads() {
        for len in [0, MAX_FRAME_BYTES as u32 + 1, u32::MAX] {
            let mut decoder = Decoder::<Request>::default();
            assert!(decoder.push(&len.to_be_bytes()).is_err());
            assert!(read_frame::<Request>(&mut len.to_be_bytes().as_slice()).is_err());
        }
        let mut frame = Frame::new(Request::new(None, Action::ListOwnedSeats));
        frame.version += 1;
        assert!(encode(&frame).is_err());
        let body = serde_json::to_vec(&frame).unwrap();
        let raw = [(body.len() as u32).to_be_bytes().as_slice(), &body].concat();
        assert!(read_frame::<Request>(&mut raw.as_slice()).is_err());
        let bytes = encode(&Frame::new(Request::new(None, Action::ListOwnedSeats))).unwrap();
        assert!(read_frame::<Request>(&mut &bytes[..bytes.len() - 1]).is_err());
    }
}
