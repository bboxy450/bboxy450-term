use super::{PendingReason as E, Result};
use ed25519_dalek::SigningKey;
use std::sync::atomic::{AtomicU8, Ordering};
use ulid::Ulid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Custody {
    DataProtection,
    LoginKeychain,
    Memory,
}
impl Custody {
    pub fn label(self) -> &'static str {
        match self {
            Self::DataProtection => "data protection",
            Self::LoginKeychain => "dev custody",
            Self::Memory => "test custody",
        }
    }
}

/// Internal IO-owner seam; never hand this store or a device key to a harness.
pub trait DeviceKeyStore {
    fn custody(&self) -> Custody;
    fn load(&self, device: Ulid) -> Result<Option<SigningKey>>;
    fn create(&self, device: Ulid, key: &SigningKey) -> Result<()>;
}

/// Data Protection Keychain, or explicit dev custody after -34018 (#17).
/// The production service is fixed; acceptance tests use an isolated service.
pub struct KeychainDeviceKeys {
    service: String,
    tier: AtomicU8,
}
impl Default for KeychainDeviceKeys {
    fn default() -> Self {
        Self {
            service: "bboxy450-term".into(),
            tier: AtomicU8::new(0),
        }
    }
}
impl KeychainDeviceKeys {
    /// Separate test service; cannot select another production service.
    pub fn isolated_test(service: &str) -> Result<Self> {
        if !service.starts_with("bboxy450-term.test.") || service.len() > 128 {
            return Err(E::InvalidProof);
        }
        Ok(Self {
            service: service.into(),
            tier: AtomicU8::new(0),
        })
    }
    #[cfg(target_os = "macos")]
    fn options(
        &self,
        device: Ulid,
        custody: Custody,
    ) -> security_framework::passwords::PasswordOptions {
        let mut options = security_framework::passwords::PasswordOptions::new_generic_password(
            &self.service,
            &device.to_string(),
        );
        if custody == Custody::DataProtection {
            options.use_protected_keychain();
        }
        options.set_access_synchronized(Some(false));
        options
    }
    #[cfg(target_os = "macos")]
    fn add_protected(&self, device: Ulid, seed: &[u8]) -> security_framework::base::Result<()> {
        use core_foundation::{
            base::TCFType, boolean::CFBoolean, data::CFData, dictionary::CFDictionary,
            string::CFString,
        };
        use security_framework::access_control::{ProtectionMode, SecAccessControl};
        use security_framework_sys::{item::*, keychain_item::SecItemAdd};
        let access = SecAccessControl::create_with_protection(
            Some(ProtectionMode::AccessibleWhenUnlockedThisDeviceOnly),
            0,
        )?;
        // SAFETY: Security.framework's CFString attribute constants live for the
        // process. The owned CF values remain alive throughout SecItemAdd.
        let pairs = unsafe {
            vec![
                (
                    CFString::wrap_under_get_rule(kSecClass),
                    CFString::wrap_under_get_rule(kSecClassGenericPassword).as_CFType(),
                ),
                (
                    CFString::wrap_under_get_rule(kSecAttrService),
                    CFString::new(&self.service).as_CFType(),
                ),
                (
                    CFString::wrap_under_get_rule(kSecAttrAccount),
                    CFString::new(&device.to_string()).as_CFType(),
                ),
                (
                    CFString::wrap_under_get_rule(kSecUseDataProtectionKeychain),
                    CFBoolean::true_value().as_CFType(),
                ),
                (
                    CFString::wrap_under_get_rule(kSecAttrSynchronizable),
                    CFBoolean::false_value().as_CFType(),
                ),
                (
                    CFString::wrap_under_get_rule(kSecAttrAccessControl),
                    access.as_CFType(),
                ),
                (
                    CFString::wrap_under_get_rule(kSecValueData),
                    CFData::from_buffer(seed).as_CFType(),
                ),
            ]
        };
        let attributes = CFDictionary::from_CFType_pairs(&pairs);
        // SAFETY: attributes owns a valid dictionary; no result object is requested.
        let status = unsafe { SecItemAdd(attributes.as_concrete_TypeRef(), std::ptr::null_mut()) };
        if status == 0 {
            Ok(())
        } else {
            Err(security_framework::base::Error::from_code(status))
        }
    }
    /// Reopen the recorded tier after restart; never migrate an existing key
    /// implicitly when app signing/entitlements change.
    pub fn resume(custody: Custody) -> Result<Self> {
        if custody == Custody::Memory {
            return Err(E::InvalidProof);
        }
        Ok(Self {
            service: "bboxy450-term".into(),
            tier: AtomicU8::new(u8::from(custody == Custody::LoginKeychain)),
        })
    }
    #[cfg(target_os = "macos")]
    fn operation<T>(
        &self,
        run: impl Fn(Custody) -> security_framework::base::Result<T>,
    ) -> Result<T> {
        let tier = self.custody();
        match run(tier) {
            Err(e) if tier == Custody::DataProtection && e.code() == -34018 => {
                self.tier.store(1, Ordering::Relaxed);
                run(Custody::LoginKeychain).map_err(|e| E::KeychainUnavailable(e.code()))
            }
            result => result.map_err(|e| E::KeychainUnavailable(e.code())),
        }
    }
    /// Only test items can be removed by this interface; production revocation
    /// retains custody and its historical attribution.
    pub fn delete_test_item(&self, device: Ulid) -> Result<()> {
        if !self.service.starts_with("bboxy450-term.test.") {
            return Err(E::InvalidProof);
        }
        #[cfg(target_os = "macos")]
        return self
            .operation(|tier| {
                if tier == Custody::LoginKeychain {
                    let chain = security_framework::os::macos::keychain::SecKeychain::default()?;
                    let (_, item) =
                        chain.find_generic_password(&self.service, &device.to_string())?;
                    item.delete();
                    return match chain.find_generic_password(&self.service, &device.to_string()) {
                        Err(e) if e.code() == -25300 => Ok(()),
                        Err(e) => Err(e),
                        Ok(_) => Err(security_framework::base::Error::from_code(-25299)),
                    };
                }
                security_framework::passwords::delete_generic_password_options(
                    self.options(device, tier),
                )
            })
            .or_else(|e| {
                if e == E::KeychainUnavailable(-25300) {
                    Ok(())
                } else {
                    Err(e)
                }
            });
        #[cfg(not(target_os = "macos"))]
        {
            let _ = device;
            Err(E::KeychainUnavailable(-4))
        }
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
        bbt_rt::assert_not_ui_thread();
        #[cfg(target_os = "macos")]
        {
            match self.operation(|tier| {
                if tier == Custody::LoginKeychain {
                    let chain = security_framework::os::macos::keychain::SecKeychain::default()?;
                    let (password, _) =
                        chain.find_generic_password(&self.service, &device.to_string())?;
                    return Ok(password.as_ref().to_vec());
                }
                security_framework::passwords::generic_password(self.options(device, tier))
            }) {
                Ok(mut bytes) => {
                    let key = <[u8; 32]>::try_from(bytes.as_slice()).map(|mut seed| {
                        let key = SigningKey::from_bytes(&seed);
                        seed.fill(0);
                        key
                    });
                    bytes.fill(0);
                    key.map(Some).map_err(|_| E::InvalidProof)
                }
                Err(E::KeychainUnavailable(-25300)) => Ok(None),
                Err(e) => Err(e),
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = device;
            Err(E::KeychainUnavailable(-4))
        }
    }
    fn create(&self, device: Ulid, key: &SigningKey) -> Result<()> {
        bbt_rt::assert_not_ui_thread();
        if device == Ulid::nil() || self.load(device)?.is_some() {
            return Err(E::JournalConflict);
        }
        #[cfg(target_os = "macos")]
        {
            let mut seed = key.to_bytes();
            let result = self.operation(|tier| {
                if tier == Custody::LoginKeychain {
                    // The legacy API explicitly selects the login keychain;
                    // adding synchronizable/DP-only SecItem attributes would
                    // route unsigned executables back to the protected store.
                    return security_framework::os::macos::keychain::SecKeychain::default()?
                        .add_generic_password(&self.service, &device.to_string(), &seed);
                }
                self.add_protected(device, &seed)
            });
            seed.fill(0);
            result
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = key;
            Err(E::KeychainUnavailable(-4))
        }
    }
}
