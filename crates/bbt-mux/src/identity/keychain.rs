use super::{PendingReason as E, Result};
use ed25519_dalek::SigningKey;
use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};
use ulid::Ulid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Custody {
    DataProtection,
    LoginKeychain,
    Memory,
    File,
}
impl Custody {
    pub fn label(self) -> &'static str {
        match self {
            Self::DataProtection => "data protection",
            Self::LoginKeychain => "dev custody",
            Self::Memory => "test custody",
            Self::File => "private file custody",
        }
    }
}

/// Internal IO-owner seam; never hand this store or a device key to a harness.
pub trait DeviceKeyStore {
    fn custody(&self) -> Custody;
    /// Describes this store, independently of the process's default selection.
    fn custody_label(&self) -> &'static str {
        self.custody().label()
    }
    fn load(&self, device: Ulid) -> Result<Option<SigningKey>>;
    fn create(&self, device: Ulid, key: &SigningKey) -> Result<()>;
    /// A still-running native access cannot admit another interactive attempt.
    fn native_busy(&self) -> bool {
        false
    }
    /// Native policy metadata without reading a secret or opening a dialog.
    fn native_acl(&self, _device: Ulid) -> Result<bbt_proto::identity::CustodyAcl> {
        Ok(bbt_proto::identity::CustodyAcl::NotNative)
    }
    /// A caller must journal the explicit act before permitting one native access.
    fn unlock(
        &self,
        _device: Ulid,
        _expected_public: [u8; 32],
        _progress: super::unlock::Progress,
    ) -> Result<bbt_proto::identity::UnlockOutcome> {
        Err(E::KeychainUnavailable(-4))
    }
    /// Resume only verified replacement cleanup; this port never permits interaction.
    fn finish_unlock(
        &self,
        _device: Ulid,
        _expected_public: [u8; 32],
        _progress: super::unlock::Progress,
    ) -> Result<bool> {
        Err(E::KeychainUnavailable(-4))
    }
}

/// Data Protection Keychain, or explicit dev custody after -34018 (#17).
/// Normal queries forbid UI; only the explicit unlock port permits one read.
#[derive(Clone)]
pub struct KeychainDeviceKeys {
    service: String,
    tier: Arc<AtomicU8>,
    io: Arc<super::io::CustodyIo>,
    #[cfg(target_os = "macos")]
    allow_fallback: bool,
}
impl Default for KeychainDeviceKeys {
    fn default() -> Self {
        Self {
            service: "bboxy450-term".into(),
            tier: Arc::new(AtomicU8::new(0)),
            io: super::io::keychain_io(),
            #[cfg(target_os = "macos")]
            allow_fallback: true,
        }
    }
}
impl KeychainDeviceKeys {
    /// A separate service for explicit native fixtures; never selected at normal startup.
    pub fn isolated_test(service: &str) -> Result<Self> {
        if !service.starts_with("bboxy450-term.test.") || service.len() > 128 {
            return Err(E::InvalidProof);
        }
        Ok(Self {
            service: service.into(),
            tier: Arc::new(AtomicU8::new(0)),
            io: super::io::keychain_io(),
            #[cfg(target_os = "macos")]
            allow_fallback: true,
        })
    }
    /// Reopen the recorded tier; never substitute file or memory custody on a failure.
    pub fn resume(custody: Custody) -> Result<Self> {
        if custody == Custody::File {
            return Err(E::FileCustodyUnavailable);
        }
        if custody == Custody::Memory {
            return Err(E::InvalidProof);
        }
        Ok(Self {
            service: "bboxy450-term".into(),
            tier: Arc::new(AtomicU8::new(u8::from(custody == Custody::LoginKeychain))),
            io: super::io::keychain_io(),
            #[cfg(target_os = "macos")]
            allow_fallback: false,
        })
    }
    #[cfg(target_os = "macos")]
    fn operation<T>(&self, run: impl Fn(Custody) -> Result<T>) -> Result<T> {
        let tier = self.custody();
        match run(tier) {
            Err(E::KeychainUnavailable(-34018))
                if self.allow_fallback && tier == Custody::DataProtection =>
            {
                self.tier.store(1, Ordering::Relaxed);
                run(Custody::LoginKeychain)
            }
            value => value,
        }
    }
    fn load_inner(&self, device: Ulid) -> Result<Option<SigningKey>> {
        bbt_rt::assert_not_ui_thread();
        #[cfg(target_os = "macos")]
        return self.operation(|tier| {
            super::unlock::read_preferred(
                &super::native_keychain::Native::new(&self.service, tier),
                &device.to_string(),
            )
        });
        #[cfg(not(target_os = "macos"))]
        {
            let _ = device;
            Err(E::KeychainUnavailable(-4))
        }
    }
    fn create_inner(&self, device: Ulid, key: &SigningKey) -> Result<()> {
        bbt_rt::assert_not_ui_thread();
        if device == Ulid::nil() || self.load_inner(device)?.is_some() {
            return Err(E::JournalConflict);
        }
        #[cfg(target_os = "macos")]
        {
            use super::unlock::Vault;
            self.operation(|tier| {
                super::native_keychain::Native::new(&self.service, tier)
                    .add(&format!("{device}.dr1"), key)
            })
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = key;
            Err(E::KeychainUnavailable(-4))
        }
    }
    /// Only explicit test service items may be removed through this legacy helper.
    pub fn delete_test_item(&self, device: Ulid) -> Result<()> {
        if !self.service.starts_with("bboxy450-term.test.") {
            return Err(E::InvalidProof);
        }
        let keys = self.clone();
        self.io.run(move || {
            #[cfg(target_os = "macos")]
            {
                use super::unlock::Vault;
                keys.operation(|tier| {
                    let native = super::native_keychain::Native::new(&keys.service, tier);
                    native.remove(&format!("{device}.dr1"))?;
                    native.remove(&device.to_string())
                })
            }
            #[cfg(not(target_os = "macos"))]
            {
                let _ = (device, keys);
                Err(E::KeychainUnavailable(-4))
            }
        })
    }
}
impl DeviceKeyStore for KeychainDeviceKeys {
    fn custody(&self) -> Custody {
        if self.tier.load(Ordering::Relaxed) == 0 {
            Custody::DataProtection
        } else {
            Custody::LoginKeychain
        }
    }
    fn load(&self, device: Ulid) -> Result<Option<SigningKey>> {
        let keys = self.clone();
        self.io.run(move || keys.load_inner(device))
    }
    fn create(&self, device: Ulid, key: &SigningKey) -> Result<()> {
        let keys = self.clone();
        let key = key.clone();
        self.io.run(move || keys.create_inner(device, &key))
    }
    fn native_busy(&self) -> bool {
        self.io.busy()
    }
    fn native_acl(&self, device: Ulid) -> Result<bbt_proto::identity::CustodyAcl> {
        let keys = self.clone();
        self.io.run(move || {
            #[cfg(target_os = "macos")]
            return keys.operation(|tier| {
                super::native_keychain::Native::new(&keys.service, tier).acl(&device.to_string())
            });
            #[cfg(not(target_os = "macos"))]
            {
                let _ = (keys, device);
                Err(E::KeychainUnavailable(-4))
            }
        })
    }
    fn unlock(
        &self,
        device: Ulid,
        expected: [u8; 32],
        progress: super::unlock::Progress,
    ) -> Result<bbt_proto::identity::UnlockOutcome> {
        let keys = self.clone();
        self.io.run(move || {
            #[cfg(target_os = "macos")]
            {
                super::native_keychain::Native::new(&keys.service, keys.custody())
                    .validate_caller()?;
                // Settle entitlement fallback with a noninteractive metadata probe FIRST.
                // Never replay an interactive read across tiers.
                match keys.operation(|tier| {
                    super::native_keychain::Native::new(&keys.service, tier)
                        .acl(&device.to_string())
                }) {
                    Ok(_) | Err(E::KeychainInteractionRequired) => {}
                    Err(error) => return Err(error),
                }
                super::unlock::migrate(
                    &super::native_keychain::Native::new(&keys.service, keys.custody()),
                    &device.to_string(),
                    &expected,
                    &progress,
                )
            }
            #[cfg(not(target_os = "macos"))]
            {
                let _ = (keys, device, expected, progress);
                Err(E::KeychainUnavailable(-4))
            }
        })
    }
    fn finish_unlock(
        &self,
        device: Ulid,
        expected: [u8; 32],
        progress: super::unlock::Progress,
    ) -> Result<bool> {
        let keys = self.clone();
        self.io.run(move || {
            #[cfg(target_os = "macos")]
            return keys.operation(|tier| {
                super::unlock::finish_migration(
                    &super::native_keychain::Native::new(&keys.service, tier),
                    &device.to_string(),
                    &expected,
                    &progress,
                )
            });
            #[cfg(not(target_os = "macos"))]
            {
                let _ = (keys, device, expected, progress);
                Err(E::KeychainUnavailable(-4))
            }
        })
    }
}

#[cfg(any(target_os = "macos", test))]
pub(super) fn keychain_error(code: i32) -> E {
    match code {
        -128 | -25293 | -25308 | -25315 => E::KeychainInteractionRequired,
        _ => E::KeychainUnavailable(code),
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "macos")]
    // keeps: spec id1-seat-chain
    #[test]
    fn native_recorded_tier_never_falls_back_after_an_entitlement_refusal() {
        use super::{Custody, DeviceKeyStore, E, KeychainDeviceKeys};
        let resumed = KeychainDeviceKeys::resume(Custody::DataProtection).unwrap();
        let calls = std::cell::Cell::new(0);
        let result: super::Result<()> = resumed.operation(|_| {
            calls.set(calls.get() + 1);
            Err(E::KeychainUnavailable(-34018))
        });
        assert_eq!(result, Err(E::KeychainUnavailable(-34018)));
        assert_eq!(calls.get(), 1);
        assert_eq!(resumed.custody(), Custody::DataProtection);
        let fresh = KeychainDeviceKeys::default();
        fresh
            .operation(|tier| {
                if tier == Custody::DataProtection {
                    Err(E::KeychainUnavailable(-34018))
                } else {
                    Ok(())
                }
            })
            .unwrap();
        assert_eq!(
            fresh.custody(),
            Custody::LoginKeychain,
            "only an unrecorded first choice may select dev custody"
        );
    }
    // keeps: spec id1-seat-chain
    #[test]
    fn denied_or_interactive_keychain_answers_have_a_named_hold() {
        for code in [-128, -25293, -25308, -25315] {
            assert_eq!(
                super::keychain_error(code),
                super::E::KeychainInteractionRequired
            );
        }
        assert_eq!(
            super::keychain_error(-34018),
            super::E::KeychainUnavailable(-34018)
        );
        assert_eq!(
            super::keychain_error(-25300),
            super::E::KeychainUnavailable(-25300)
        );
    }
}
