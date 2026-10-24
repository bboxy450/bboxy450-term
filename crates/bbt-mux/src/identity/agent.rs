//! Narrow Assuan client. Each serialized transaction has a fresh connection;
//! errors close it, so no hash/key selection can leak into a later signature.
use std::{
    io::{BufReader, Read, Write},
    os::unix::{
        fs::{FileTypeExt, MetadataExt},
        net::UnixStream,
    },
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use ed25519_dalek::{Signature, VerifyingKey};

use super::{PendingReason as E, Result, hex};

pub(crate) struct Agent {
    pub socket: PathBuf,
    pub grip: String,
    pub public: VerifyingKey,
    pub timeout: Duration,
    pub cancelled: AtomicBool,
    serial: Mutex<()>,
}
pub(crate) enum Input<'a> {
    Raw(&'a [u8]),
    Sha256(&'a [u8]),
}

/// The existing root signing contract; a test root is held in its own file store.
pub(crate) trait RootSigner {
    fn sign_root(&self, input: Input<'_>) -> Result<[u8; 64]>;
}
impl RootSigner for Agent {
    fn sign_root(&self, input: Input<'_>) -> Result<[u8; 64]> {
        self.sign(input)
    }
}
impl RootSigner for ed25519_dalek::SigningKey {
    fn sign_root(&self, input: Input<'_>) -> Result<[u8; 64]> {
        use ed25519_dalek::Signer;
        if matches!(&input, Input::Sha256(bytes) if bytes.len() != 32) {
            return Err(E::InvalidProof);
        }
        let (Input::Raw(bytes) | Input::Sha256(bytes)) = input;
        if bytes.is_empty() || bytes.len() > 64 * 1024 {
            return Err(E::InvalidProof);
        }
        Ok(self.sign(bytes).to_bytes())
    }
}

impl Agent {
    pub fn new(
        socket: PathBuf,
        grip: String,
        public: VerifyingKey,
        timeout: Duration,
    ) -> Result<Self> {
        if grip.len() != 40 || !grip.bytes().all(|b| b.is_ascii_hexdigit()) || timeout.is_zero() {
            return Err(E::InvalidProof);
        }
        Ok(Self {
            socket,
            grip,
            public,
            timeout,
            cancelled: AtomicBool::new(false),
            serial: Mutex::new(()),
        })
    }

    pub fn sign(&self, input: Input<'_>) -> Result<[u8; 64]> {
        bbt_rt::assert_not_ui_thread();
        let _lock = self.serial.lock().map_err(|_| E::AgentUnavailable)?;
        if self.cancelled.load(Ordering::Relaxed) {
            return Err(E::Cancelled);
        }
        let data = match input {
            Input::Raw(data) | Input::Sha256(data) => data,
        };
        if data.is_empty() || data.len() > 64 * 1024 {
            return Err(E::InvalidProof);
        }
        let meta = std::fs::symlink_metadata(&self.socket).map_err(|_| E::AgentUnavailable)?;
        if !meta.file_type().is_socket() || meta.uid() != unsafe { libc::geteuid() } {
            return Err(E::AgentUnavailable);
        }
        let stream = UnixStream::connect(&self.socket).map_err(|_| E::AgentUnavailable)?;
        stream
            .set_write_timeout(Some(self.timeout))
            .map_err(|_| E::AgentUnavailable)?;
        let mut io = Session {
            reader: BufReader::new(stream),
            deadline: Instant::now() + self.timeout,
            cancelled: &self.cancelled,
        };
        io.response(None)?;
        io.command("RESET", None)?;
        let version = io.command("GETINFO version", None)?;
        if !version.starts_with(b"2.5.") {
            return Err(E::UnsupportedAgent);
        }
        // READKEY is public-only. Check the selected keygrip before requesting
        // any signature, then independently verify every returned signature.
        let public = io.command(&format!("READKEY {}", self.grip), None)?;
        let tree = sexp(&public)?;
        let fields = tree.list()?;
        if fields.len() != 2 || fields[0].atom()? != b"public-key" {
            return Err(E::InvalidProof);
        }
        let ecc = fields[1].list()?;
        if ecc.first().ok_or(E::InvalidProof)?.atom()? != b"ecc" {
            return Err(E::InvalidProof);
        }
        let q = ecc
            .iter()
            .skip(1)
            .find_map(|field| {
                let pair = field.list().ok()?;
                (pair.len() == 2 && pair[0].atom().ok()? == b"q")
                    .then(|| pair[1].atom().ok())
                    .flatten()
            })
            .ok_or(E::InvalidProof)?;
        if q.len() != 33 || q[0] != 0x40 || q[1..] != self.public.to_bytes() {
            return Err(E::FingerprintMismatch);
        }
        io.command(&format!("SIGKEY {}", self.grip), None)?;
        match input {
            Input::Raw(_) => {
                io.command("SETHASH --inquire", Some(data))?;
            }
            Input::Sha256(_) => {
                if data.len() != 32 {
                    return Err(E::InvalidProof);
                }
                io.command(&format!("SETHASH 8 {}", hex(data)), None)?;
            }
        }
        let encoded = io.command("PKSIGN", None)?;
        let sig = signature(&encoded)?;
        self.public
            .verify_strict(data, &Signature::from_bytes(&sig))
            .map_err(|_| E::InvalidProof)?;
        Ok(sig)
    }
}

struct Session<'a> {
    reader: BufReader<UnixStream>,
    deadline: Instant,
    cancelled: &'a AtomicBool,
}
impl Session<'_> {
    fn line(&mut self) -> Result<Vec<u8>> {
        let mut line = Vec::new();
        loop {
            if self.cancelled.load(Ordering::Relaxed) {
                return Err(E::Cancelled);
            }
            let remaining = self
                .deadline
                .checked_duration_since(Instant::now())
                .ok_or(E::TimedOut)?;
            self.reader
                .get_ref()
                .set_read_timeout(Some(remaining.min(Duration::from_millis(100))))
                .map_err(|_| E::AgentUnavailable)?;
            let mut byte = [0];
            match self.reader.read(&mut byte) {
                Ok(0) => return Err(E::AgentUnavailable),
                Ok(_) if byte[0] == b'\n' => return Ok(line),
                Ok(_) => {
                    line.push(byte[0]);
                    if line.len() > 1000 {
                        return Err(E::InvalidProof);
                    }
                }
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock
                            | std::io::ErrorKind::TimedOut
                            | std::io::ErrorKind::Interrupted
                    ) =>
                {
                    continue;
                }
                Err(_) => return Err(E::AgentUnavailable),
            }
        }
    }
    fn command(&mut self, command: &str, inquiry: Option<&[u8]>) -> Result<Vec<u8>> {
        if self.cancelled.load(Ordering::Relaxed) {
            return Err(E::Cancelled);
        }
        let remaining = self
            .deadline
            .checked_duration_since(Instant::now())
            .ok_or(E::TimedOut)?;
        self.reader
            .get_ref()
            .set_write_timeout(Some(remaining))
            .map_err(|_| E::AgentUnavailable)?;
        self.reader
            .get_mut()
            .write_all(format!("{command}\n").as_bytes())
            .map_err(|_| E::AgentUnavailable)?;
        self.response(inquiry)
    }
    fn response(&mut self, mut inquiry: Option<&[u8]>) -> Result<Vec<u8>> {
        let mut data = Vec::new();
        for _ in 0..256 {
            let line = self.line()?;
            if line == b"OK" || line.starts_with(b"OK ") {
                return Ok(data);
            }
            if let Some(error) = line.strip_prefix(b"ERR ") {
                let code = std::str::from_utf8(error)
                    .ok()
                    .and_then(|s| s.split_whitespace().next()?.parse::<u32>().ok())
                    .ok_or(E::InvalidProof)?;
                return Err(if matches!(code & 0xffff, 99 | 198) {
                    E::Cancelled
                } else {
                    E::AgentRejected(code)
                });
            }
            if let Some(chunk) = line.strip_prefix(b"D ") {
                data.extend(unescape(chunk)?);
                if data.len() > 16 * 1024 {
                    return Err(E::InvalidProof);
                }
            } else if line == b"INQUIRE TBSDATA" || line.starts_with(b"INQUIRE TBSDATA ") {
                let bytes = inquiry.take().ok_or(E::UnsupportedAgent)?;
                for chunk in bytes.chunks(200) {
                    let mut encoded = b"D ".to_vec();
                    for &b in chunk {
                        if (32..=126).contains(&b) && b != b'%' {
                            encoded.push(b);
                        } else {
                            encoded.extend_from_slice(format!("%{b:02X}").as_bytes());
                        }
                    }
                    encoded.push(b'\n');
                    self.reader
                        .get_mut()
                        .write_all(&encoded)
                        .map_err(|_| E::AgentUnavailable)?;
                }
                self.reader
                    .get_mut()
                    .write_all(b"END\n")
                    .map_err(|_| E::AgentUnavailable)?;
            } else if !(line.starts_with(b"S ") || line.starts_with(b"#") || line.is_empty()) {
                return Err(E::UnsupportedAgent);
            }
        }
        Err(E::InvalidProof)
    }
}
fn unescape(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut result = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = bytes.get(i + 1..i + 3).ok_or(E::InvalidProof)?;
            result.push(
                u8::from_str_radix(std::str::from_utf8(hex).map_err(|_| E::InvalidProof)?, 16)
                    .map_err(|_| E::InvalidProof)?,
            );
            i += 3;
        } else {
            result.push(bytes[i]);
            i += 1;
        }
    }
    Ok(result)
}

enum Node<'a> {
    Atom(&'a [u8]),
    List(Vec<Node<'a>>),
}
impl<'a> Node<'a> {
    fn atom(&self) -> Result<&'a [u8]> {
        match self {
            Self::Atom(a) => Ok(a),
            _ => Err(E::InvalidProof),
        }
    }
    fn list(&self) -> Result<&[Self]> {
        match self {
            Self::List(a) => Ok(a),
            _ => Err(E::InvalidProof),
        }
    }
}
fn sexp(bytes: &[u8]) -> Result<Node<'_>> {
    fn parse<'a>(bytes: &mut &'a [u8], depth: usize) -> Result<Node<'a>> {
        if depth > 8 {
            return Err(E::InvalidProof);
        }
        if bytes.first() == Some(&b'(') {
            *bytes = &bytes[1..];
            let mut children = Vec::new();
            while bytes.first() != Some(&b')') {
                if children.len() >= 16 || bytes.is_empty() {
                    return Err(E::InvalidProof);
                }
                children.push(parse(bytes, depth + 1)?);
            }
            *bytes = &bytes[1..];
            Ok(Node::List(children))
        } else {
            let end = bytes
                .iter()
                .position(|b| *b == b':')
                .filter(|end| *end > 0 && *end < 5)
                .ok_or(E::InvalidProof)?;
            if !bytes[..end].iter().all(u8::is_ascii_digit) {
                return Err(E::InvalidProof);
            }
            let len: usize = std::str::from_utf8(&bytes[..end])
                .map_err(|_| E::InvalidProof)?
                .parse()
                .map_err(|_| E::InvalidProof)?;
            let atom = bytes.get(end + 1..end + 1 + len).ok_or(E::InvalidProof)?;
            *bytes = &bytes[end + 1 + len..];
            Ok(Node::Atom(atom))
        }
    }
    let mut rest = bytes;
    let result = parse(&mut rest, 0)?;
    if !rest.is_empty() {
        return Err(E::InvalidProof);
    }
    Ok(result)
}
fn signature(bytes: &[u8]) -> Result<[u8; 64]> {
    let root = sexp(bytes)?;
    let root = root.list()?;
    if root.len() != 2 || root[0].atom()? != b"sig-val" {
        return Err(E::InvalidProof);
    }
    let alg = root[1].list()?;
    if alg.len() != 3 || alg[0].atom()? != b"eddsa" {
        return Err(E::InvalidProof);
    }
    let mut signature = [0; 64];
    for (i, name) in [b"r", b"s"].iter().enumerate() {
        let pair = alg[i + 1].list()?;
        if pair.len() != 2 || pair[0].atom()? != *name || pair[1].atom()?.len() != 32 {
            return Err(E::InvalidProof);
        }
        signature[i * 32..i * 32 + 32].copy_from_slice(pair[1].atom()?);
    }
    Ok(signature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::BufRead, os::unix::net::UnixListener};

    fn mock(reply: &'static [u8]) -> (tempfile::TempDir, Agent, std::thread::JoinHandle<()>) {
        let dir = tempfile::Builder::new()
            .prefix("id1a-agent-")
            .tempdir_in("/tmp")
            .unwrap();
        let path = dir.path().join("agent.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let public = ed25519_dalek::SigningKey::from_bytes(&[7; 32]).verifying_key();
        let agent = Agent::new(path, "A".repeat(40), public, Duration::from_secs(1)).unwrap();
        let worker = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream.write_all(b"OK mock\n").unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            assert_eq!(line, "RESET\n");
            stream.write_all(reply).unwrap();
            // Keep the read half open until the client consumes the response;
            // closing an unread socket can turn the intended error into reset.
            line.clear();
            let _ = reader.read_line(&mut line);
        });
        (dir, agent, worker)
    }
    #[test]
    fn agent_cancellation_and_bad_framing_fail_closed() {
        let (_dir, agent, worker) = mock(b"ERR 83886179 Operation cancelled\n");
        assert_eq!(agent.sign(Input::Raw(b"typed request")), Err(E::Cancelled));
        worker.join().unwrap();
        let (_dir, agent, worker) = mock(b"INQUIRE PASSPHRASE\n");
        assert_eq!(
            agent.sign(Input::Raw(b"typed request")),
            Err(E::UnsupportedAgent)
        );
        worker.join().unwrap();
    }
    #[test]
    fn agent_deadline_is_bounded() {
        let dir = tempfile::Builder::new()
            .prefix("id1a-timeout-")
            .tempdir_in("/tmp")
            .unwrap();
        let path = dir.path().join("agent.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let agent = Agent::new(
            path,
            "A".repeat(40),
            ed25519_dalek::SigningKey::from_bytes(&[7; 32]).verifying_key(),
            Duration::from_millis(80),
        )
        .unwrap();
        let worker = std::thread::spawn(move || {
            let (_stream, _) = listener.accept().unwrap();
            std::thread::sleep(Duration::from_millis(160));
        });
        assert_eq!(agent.sign(Input::Raw(b"typed request")), Err(E::TimedOut));
        worker.join().unwrap();
    }
    #[test]
    fn malformed_agent_data_is_rejected() {
        for data in [b"%".as_slice(), b"%GG", b"%0"] {
            assert!(unescape(data).is_err());
        }
        for data in [
            b"(7:sig-val(3:rsa(1:s1:x)))".as_slice(),
            b"9999:x",
            b"(7:sig-val(5:eddsa(1:r1:x)(1:s1:y)))",
            b"(1:x)junk",
        ] {
            assert!(signature(data).is_err());
        }
    }
}
