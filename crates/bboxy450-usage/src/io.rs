// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use crate::*;
use std::{
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};
const MAX_BYTES: u64 = 1024 * 1024;

pub struct LiveReader {
    pub home: PathBuf,
}
#[allow(
    clippy::disallowed_methods,
    reason = "read-only OS IO runs in the poller's bounded spawn_blocking slot, never the UI"
)]
impl CredentialReader for LiveReader {
    fn env(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }
    fn home(&self) -> PathBuf {
        self.home.clone()
    }
    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
    fn file(&self, path: &Path) -> Result<Option<Vec<u8>>, Failure> {
        let file = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(Failure::Credentials),
        };
        if !file.metadata().map_err(|_| Failure::Credentials)?.is_file() {
            return Err(Failure::Credentials);
        }
        let mut data = Vec::new();
        file.take(MAX_BYTES + 1)
            .read_to_end(&mut data)
            .map_err(|_| Failure::Credentials)?;
        if data.len() as u64 > MAX_BYTES {
            return Err(Failure::Credentials);
        }
        Ok(Some(data))
    }
    fn keychain(&self, service: &str) -> Result<Option<Vec<u8>>, Failure> {
        keychain(service)
    }
    fn rollout_tail(&self, root: &Path) -> Option<Vec<u8>> {
        // Newest-first YYYY/MM/DD directories, capped traversal and five bounded tails.
        // Never recursively scan the user's entire session history.
        fn children(path: &Path, budget: &mut usize) -> Vec<PathBuf> {
            let mut paths = Vec::new();
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.take(*budget) {
                    *budget = budget.saturating_sub(1);
                    if let Ok(entry) = entry {
                        paths.push(entry.path());
                    }
                }
            }
            paths.sort_unstable_by(|a, b| b.cmp(a));
            paths
        }
        let mut budget = 4096;
        let mut dirs = vec![root.join("sessions")];
        for _ in 0..3 {
            dirs = dirs
                .iter()
                .flat_map(|d| children(d, &mut budget))
                .take(64)
                .collect();
        }
        let mut files = Vec::new();
        for day in dirs {
            for path in children(&day, &mut budget) {
                if path.extension().is_some_and(|s| s == "jsonl") {
                    let modified = path.metadata().ok().and_then(|m| m.modified().ok());
                    files.push((modified, path));
                }
            }
            if files.len() >= 5 || budget == 0 {
                break;
            }
        }
        files.sort_unstable_by_key(|a| std::cmp::Reverse(a.0));
        for (_, path) in files.into_iter().take(5) {
            let mut file = std::fs::File::open(path).ok()?;
            let size = file.metadata().ok()?.len();
            file.seek(SeekFrom::Start(size.saturating_sub(256 * 1024)))
                .ok()?;
            let mut bytes = Vec::new();
            file.take(256 * 1024).read_to_end(&mut bytes).ok()?;
            if crate::codex::rollout(&bytes).is_some() {
                return Some(bytes);
            }
        }
        None
    }
}
pub(crate) fn epoch_millis() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
        * 1000.
}

#[cfg(not(target_os = "macos"))]
fn keychain(_: &str) -> Result<Option<Vec<u8>>, Failure> {
    Ok(None)
}

#[cfg(target_os = "macos")]
fn keychain(service: &str) -> Result<Option<Vec<u8>>, Failure> {
    use core_foundation::{
        base::{CFType, TCFType},
        boolean::CFBoolean,
        data::CFData,
        dictionary::CFDictionary,
        string::CFString,
    };
    use security_framework_sys::item::*;
    use security_framework_sys::keychain_item::SecItemCopyMatching;
    #[link(name = "Security", kind = "framework")]
    unsafe extern "C" {
        static kSecUseAuthenticationUIFail: core_foundation::string::CFStringRef;
    }
    // Interaction is forbidden in the query itself. Never spawn `security`, unlock or refresh.
    unsafe {
        let s = |p| CFString::wrap_under_get_rule(p).into_CFType();
        let query = CFDictionary::<CFType, CFType>::from_CFType_pairs(&[
            (s(kSecClass), s(kSecClassGenericPassword)),
            (s(kSecAttrService), CFString::new(service).into_CFType()),
            (s(kSecReturnData), CFBoolean::true_value().into_CFType()),
            (s(kSecUseAuthenticationUI), s(kSecUseAuthenticationUIFail)),
        ]);
        let mut result = std::ptr::null();
        let status = SecItemCopyMatching(query.as_concrete_TypeRef(), &mut result);
        match status {
            0 if !result.is_null() => {
                let data = CFData::wrap_under_create_rule(result.cast());
                if data.len() as u64 > MAX_BYTES {
                    return Err(Failure::Credentials);
                }
                Ok(Some(data.bytes().to_vec()))
            }
            -25300 => Ok(None), // errSecItemNotFound
            _ => Err(Failure::KeychainLocked),
        }
    }
}

pub struct Http {
    client: reqwest::Client,
}
#[derive(Default)]
pub struct LiveHttp(std::sync::OnceLock<Result<Http, Failure>>);
impl Fetch for LiveHttp {
    fn get(&self, request: Request) -> FetchFuture<'_> {
        Box::pin(async move {
            match self.0.get_or_init(Http::new) {
                Ok(http) => http.get(request).await,
                Err(e) => Err(*e),
            }
        })
    }
}
impl Http {
    pub fn new() -> Result<Self, Failure> {
        reqwest::Client::builder()
            .timeout(READ_TIMEOUT)
            .connect_timeout(std::time::Duration::from_secs(3))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map(|client| Self { client })
            .map_err(|_| Failure::Network)
    }
}
impl Fetch for Http {
    fn get(&self, request: Request) -> FetchFuture<'_> {
        Box::pin(async move {
            let mut builder = self.client.get(request.url);
            for (name, value) in request.headers {
                builder = builder.header(name, value);
            }
            let mut response = builder.send().await.map_err(|_| Failure::Network)?;
            let status = response.status().as_u16();
            if !(200..300).contains(&status) {
                return Ok(Response {
                    status,
                    body: Vec::new(),
                });
            }
            let mut body = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(|_| Failure::Network)? {
                if body.len().saturating_add(chunk.len()) > MAX_BYTES as usize {
                    return Err(Failure::Decode);
                }
                body.extend_from_slice(&chunk);
            }
            Ok(Response { status, body })
        })
    }
}
