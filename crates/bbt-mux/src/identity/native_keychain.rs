//! Native calls remain in mux for this cut. Every query states its interaction policy.
use core_foundation::{
    array::CFArray,
    base::{CFType, CFTypeRef, TCFType},
    boolean::CFBoolean,
    data::{CFData, CFDataRef},
    dictionary::CFDictionary,
    string::{CFString, CFStringRef},
    url::CFURL,
};
use ed25519_dalek::SigningKey;
use security_framework::{
    access_control::{ProtectionMode, SecAccessControl},
    os::macos::{
        code_signing::{Flags, SecCode, SecRequirement, SecStaticCode},
        keychain::SecKeychain,
    },
};
use security_framework_sys::{
    base::SecAccessRef,
    code_signing::{SecRequirementRef, SecStaticCodeRef},
    item::*,
    keychain_item::{SecItemAdd, SecItemCopyMatching, SecItemDelete},
};
use std::ptr;

use super::{Custody, PendingReason as E, Result, keychain::keychain_error, unlock::Vault};

/// The development runner and bundle recipe install this same explicit requirement.
pub(super) const REQUIREMENT: &str = "identifier \"com.bboxy450.term\" and anchor apple generic and certificate leaf[subject.OU] = \"UQB3368A84\"";

// Public Security.framework declarations absent from security-framework-sys 2.17.
unsafe extern "C" {
    static kSecUseAuthenticationUIAllow: CFStringRef;
    static kSecUseAuthenticationUIFail: CFStringRef;
    static kSecAttrAccess: CFStringRef;
    static kSecMatchLimitOne: CFStringRef;
    fn SecAccessCreate(
        description: CFStringRef,
        applications: core_foundation::array::CFArrayRef,
        access: *mut SecAccessRef,
    ) -> i32;
    fn SecCodeCopyDesignatedRequirement(
        code: SecStaticCodeRef,
        flags: u32,
        requirement: *mut SecRequirementRef,
    ) -> i32;
    fn SecRequirementCopyData(
        requirement: SecRequirementRef,
        flags: u32,
        data: *mut CFDataRef,
    ) -> i32;
}

fn cvt(code: i32) -> Result<()> {
    if code == 0 {
        Ok(())
    } else {
        Err(keychain_error(code))
    }
}
fn pair(key: CFStringRef, value: CFType) -> (CFString, CFType) {
    // SAFETY: every caller passes one of Security.framework's static CFString keys.
    (unsafe { CFString::wrap_under_get_rule(key) }, value)
}
fn value(value: CFStringRef) -> CFType {
    // SAFETY: every caller passes one of Security.framework's static CFString values.
    unsafe { CFString::wrap_under_get_rule(value).as_CFType() }
}

pub(super) struct Native<'a> {
    service: &'a str,
    tier: Custody,
    #[cfg(test)]
    fixture: Option<FixtureAccess<'a>>,
}
#[cfg(test)]
struct FixtureAccess<'a> {
    keychain: &'a SecKeychain,
    requirement: &'a str,
}
#[cfg(test)]
const FIXTURE_ROOT_ENV: &str = "BBOXY_NATIVE_KEYCHAIN_FIXTURE_ROOT";

impl<'a> Native<'a> {
    pub(super) fn new(service: &'a str, tier: Custody) -> Self {
        Self {
            service,
            tier,
            #[cfg(test)]
            fixture: None,
        }
    }
    pub(super) fn validate_caller(&self) -> Result<()> {
        self.access().map(drop)
    }

    fn common(&self, account: &str, interactive: bool) -> Vec<(CFString, CFType)> {
        // SAFETY: Security.framework constants are valid for the process lifetime.
        unsafe {
            vec![
                pair(kSecClass, value(kSecClassGenericPassword)),
                pair(kSecAttrService, CFString::new(self.service).as_CFType()),
                pair(kSecAttrAccount, CFString::new(account).as_CFType()),
                pair(kSecAttrSynchronizable, CFBoolean::false_value().as_CFType()),
                pair(
                    kSecUseDataProtectionKeychain,
                    if self.tier == Custody::DataProtection {
                        CFBoolean::true_value()
                    } else {
                        CFBoolean::false_value()
                    }
                    .as_CFType(),
                ),
                pair(
                    kSecUseAuthenticationUI,
                    value(if interactive {
                        kSecUseAuthenticationUIAllow
                    } else {
                        kSecUseAuthenticationUIFail
                    }),
                ),
            ]
        }
    }
    fn query(
        &self,
        account: &str,
        interactive: bool,
        add: bool,
    ) -> Result<Vec<(CFString, CFType)>> {
        // A fixture child must never fall through to either operator store,
        // including the implicit Data Protection store.
        #[cfg(test)]
        if std::env::var_os(FIXTURE_ROOT_ENV).is_some()
            && (self.fixture.is_none() || self.tier != Custody::LoginKeychain)
        {
            return Err(E::InvalidProof);
        }
        let mut pairs = self.common(account, interactive);
        if self.tier == Custody::LoginKeychain {
            let keychain = self.legacy_keychain()?;
            // SAFETY: the static attribute chooses this exact login keychain, not DP.
            unsafe {
                if add {
                    pairs.push(pair(kSecUseKeychain, keychain.as_CFType()));
                } else {
                    pairs.push(pair(
                        kSecMatchSearchList,
                        CFArray::from_CFTypes(&[keychain]).as_CFType(),
                    ));
                }
            }
        }
        Ok(pairs)
    }
    fn legacy_keychain(&self) -> Result<SecKeychain> {
        #[cfg(test)]
        if let Some(fixture) = &self.fixture {
            return Ok(fixture.keychain.clone());
        }
        SecKeychain::default().map_err(|e| keychain_error(e.code()))
    }
    fn lookup(&self, account: &str, interactive: bool, data: bool) -> Result<Option<CFType>> {
        let mut pairs = self.query(account, interactive, false)?;
        // SAFETY: these static attributes request either secret bytes or public metadata.
        unsafe {
            pairs.push(pair(
                if data {
                    kSecReturnData
                } else {
                    kSecReturnAttributes
                },
                CFBoolean::true_value().as_CFType(),
            ));
            pairs.push(pair(kSecMatchLimit, value(kSecMatchLimitOne)));
        }
        let query = CFDictionary::from_CFType_pairs(&pairs);
        let mut result: CFTypeRef = ptr::null();
        // SAFETY: query owns every value for the duration of the call; result is retained.
        let code = unsafe { SecItemCopyMatching(query.as_concrete_TypeRef(), &mut result) };
        if code == -25300 {
            return Ok(None);
        }
        if interactive && code == -128 {
            return Err(E::Cancelled);
        }
        cvt(code)?;
        if result.is_null() {
            return Err(E::InvalidProof);
        }
        // SAFETY: a successful Copy operation returns one retained CF object.
        Ok(Some(unsafe { CFType::wrap_under_create_rule(result) }))
    }
    pub(super) fn acl(&self, account: &str) -> Result<bbt_proto::identity::CustodyAcl> {
        use bbt_proto::identity::CustodyAcl as A;
        if self
            .lookup(&format!("{account}.dr1"), false, false)?
            .is_some()
        {
            // The public API's opaque trusted-app data is not a readback proof of the
            // full requirement. Do not infer an ACL from a label or a successful read.
            return Ok(A::Unknown);
        }
        Ok(if self.lookup(account, false, false)?.is_some() {
            A::Legacy
        } else {
            A::Unknown
        })
    }
    fn access(&self) -> Result<CFType> {
        let requirement = REQUIREMENT;
        #[cfg(test)]
        let requirement = self
            .fixture
            .as_ref()
            .map_or(requirement, |fixture| fixture.requirement);
        let expected: SecRequirement = requirement.parse().map_err(|_| E::InvalidProof)?;
        SecCode::for_self(Flags::NONE)
            .and_then(|code| code.check_validity(Flags::NONE, &expected))
            .map_err(|_| E::KeychainSigningIdentityRequired)?;
        let path = std::env::current_exe().map_err(|_| E::StorageUnavailable)?;
        let path = CFURL::from_path(&path, false).ok_or(E::StorageUnavailable)?;
        let code = SecStaticCode::from_path(&path, Flags::NONE)
            .map_err(|_| E::KeychainSigningIdentityRequired)?;
        let mut actual = ptr::null_mut();
        // SAFETY: code is a live static code object; the Copy result is retained.
        cvt(unsafe {
            SecCodeCopyDesignatedRequirement(code.as_concrete_TypeRef(), 0, &mut actual)
        })?;
        if actual.is_null() {
            return Err(E::InvalidProof);
        }
        // SAFETY: the successful Copy call returned a SecRequirement object.
        let actual = unsafe { SecRequirement::wrap_under_create_rule(actual) };
        let encoded = |requirement: &SecRequirement| -> Result<CFData> {
            let mut data = ptr::null();
            // SAFETY: requirement is live; the result is a retained CFData object.
            cvt(unsafe {
                SecRequirementCopyData(requirement.as_concrete_TypeRef(), 0, &mut data)
            })?;
            if data.is_null() {
                return Err(E::InvalidProof);
            }
            // SAFETY: successful Copy operation returns retained CFData.
            Ok(unsafe { CFData::wrap_under_create_rule(data) })
        };
        if encoded(&actual)?.bytes() != encoded(&expected)?.bytes() {
            return Err(E::KeychainSigningIdentityRequired);
        }
        let description = CFString::new("bboxy450 team designated requirement v1");
        let mut access = ptr::null_mut();
        // SAFETY: a NULL trusted list selects the calling app. Its exact DR was checked
        // above; Keychain tracks that DR, not the path or the current CodeDirectory hash.
        cvt(unsafe {
            SecAccessCreate(description.as_concrete_TypeRef(), ptr::null(), &mut access)
        })?;
        if access.is_null() {
            return Err(E::InvalidProof);
        }
        // SAFETY: SecAccessCreate returned one retained CF access object.
        Ok(unsafe { CFType::wrap_under_create_rule(access.cast()) })
    }
}
impl Vault for Native<'_> {
    fn read(&self, account: &str, interactive: bool) -> Result<Option<SigningKey>> {
        let Some(value) = self.lookup(account, interactive, true)? else {
            return Ok(None);
        };
        let data = value.downcast::<CFData>().ok_or(E::InvalidProof)?;
        let mut seed: [u8; 32] = data.bytes().try_into().map_err(|_| E::InvalidProof)?;
        let key = SigningKey::from_bytes(&seed);
        seed.fill(0);
        Ok(Some(key))
    }
    fn add(&self, account: &str, key: &SigningKey) -> Result<()> {
        let access = self.access()?;
        let mut pairs = self.query(account, false, true)?;
        if self.tier == Custody::DataProtection {
            // Existing DP custody uses access groups; SecAccess is for the login store.
            // Preserve that tier rather than silently moving an enrolled device.
            let control = SecAccessControl::create_with_protection(
                Some(ProtectionMode::AccessibleWhenUnlockedThisDeviceOnly),
                0,
            )
            .map_err(|e| keychain_error(e.code()))?;
            // SAFETY: static attribute and owned access control remain alive in pairs.
            unsafe {
                pairs.push(pair(kSecAttrAccessControl, control.as_CFType()));
            }
        } else {
            // SAFETY: static attribute and the validated caller access object are live.
            unsafe {
                pairs.push(pair(kSecAttrAccess, access));
            }
        }
        let mut seed = key.to_bytes();
        // SAFETY: the dictionary owns its CFData copy until SecItemAdd returns.
        unsafe {
            pairs.push(pair(kSecValueData, CFData::from_buffer(&seed).as_CFType()));
        }
        seed.fill(0);
        let attributes = CFDictionary::from_CFType_pairs(&pairs);
        // SAFETY: attributes is live and no result object is requested. SecItemAdd is
        // create-only; a duplicate cannot overwrite an existing item.
        let code = unsafe { SecItemAdd(attributes.as_concrete_TypeRef(), ptr::null_mut()) };
        if code == -25299 {
            return Err(E::JournalConflict);
        }
        cvt(code)
    }
    fn remove(&self, account: &str) -> Result<()> {
        let query = CFDictionary::from_CFType_pairs(&self.query(account, false, false)?);
        // SAFETY: query owns all values; it always forbids interactive access.
        let code = unsafe { SecItemDelete(query.as_concrete_TypeRef()) };
        if code == -25300 {
            return Ok(());
        }
        cvt(code)
    }
}

#[cfg(test)]
mod fixtures;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn queries_allow_ui_only_when_the_explicit_argument_says_so() {
        for tier in [Custody::DataProtection, Custody::LoginKeychain] {
            let store = Native::new("bboxy450-term.test.query-only", tier);
            for interactive in [false, true, false] {
                let pairs = store.common("fixture", interactive);
                // SAFETY: the static key and values live for the process lifetime.
                let key = unsafe { CFString::wrap_under_get_rule(kSecUseAuthenticationUI) };
                let expected = unsafe {
                    value(if interactive {
                        kSecUseAuthenticationUIAllow
                    } else {
                        kSecUseAuthenticationUIFail
                    })
                };
                assert_eq!(
                    pairs
                        .iter()
                        .find(|(candidate, _)| *candidate == key)
                        .unwrap()
                        .1,
                    expected
                );
            }
        }
    }
    #[test]
    fn the_team_and_application_requirement_parses_without_accessing_any_keychain() {
        let _: SecRequirement = REQUIREMENT.parse().unwrap();
        assert!(REQUIREMENT.contains("UQB3368A84"));
        assert!(REQUIREMENT.contains("com.bboxy450.term"));
        assert!(!REQUIREMENT.contains("cdhash"));
    }
}
