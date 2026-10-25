//! A real native boundary under a disposable store and an ad-hoc test identity.
#![allow(clippy::disallowed_methods, reason = "owned native night fixture IO")]

use super::*;
use std::{
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::PathBuf,
    process::{Output, Stdio},
    time::Duration,
};
use tokio::{io::AsyncReadExt, process::Command};

const IDENTIFIER: &str = "com.bboxy450.native-custody-fixture";
const FIXTURE_REQUIREMENT: &str = "identifier \"com.bboxy450.native-custody-fixture\"";
const NIGHT: &str =
    "identity::native_keychain::fixtures::private_native_keychain_round_trip_and_cleanup";

unsafe extern "C" {
    fn SecKeychainDelete(keychain: security_framework_sys::base::SecKeychainRef) -> i32;
}

struct PrivateKeychain {
    keychain: SecKeychain,
    deleted: bool,
}
impl PrivateKeychain {
    fn delete(mut self) {
        // SAFETY: this is only the retained object returned when this fixture
        // created its own keychain, never an object from the default/search list.
        let code = unsafe { SecKeychainDelete(self.keychain.as_concrete_TypeRef()) };
        assert_eq!(code, 0, "private fixture keychain cleanup failed");
        self.deleted = true;
    }
}
impl Drop for PrivateKeychain {
    fn drop(&mut self) {
        if !self.deleted {
            // SAFETY: same exclusively owned keychain as the checked path above.
            let _ = unsafe { SecKeychainDelete(self.keychain.as_concrete_TypeRef()) };
        }
    }
}

async fn output(command: &mut Command, kind: &str) -> Output {
    command
        .kill_on_drop(true)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
    println!("native fixture process: {kind} pid={}", child.id().unwrap());
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let read = |pipe: tokio::process::ChildStdout| async move {
        let mut bytes = Vec::new();
        pipe.take(65_537).read_to_end(&mut bytes).await.unwrap();
        bytes
    };
    let stdout = tokio::spawn(read(stdout));
    let stderr = tokio::spawn(async move {
        let mut bytes = Vec::new();
        stderr.take(65_537).read_to_end(&mut bytes).await.unwrap();
        bytes
    });
    // A hang guard, not a speed assertion. Keep the child handle so timeout
    // cleanup kills and reaps this exact process without a PID-reuse race.
    let status = match tokio::time::timeout(Duration::from_secs(120), child.wait()).await {
        Ok(status) => status.unwrap(),
        Err(_) => {
            child.kill().await.unwrap();
            panic!("owned {kind} fixture hung");
        }
    };
    let stdout = stdout.await.unwrap();
    let stderr = stderr.await.unwrap();
    assert!(stdout.len() <= 65_536 && stderr.len() <= 65_536);
    Output {
        status,
        stdout,
        stderr,
    }
}

// keeps: spec id1-seat-chain §6.4 — private native storage boundary
#[tokio::test]
#[ignore = "night: real native calls with an owned private keychain and ad-hoc helper"]
async fn private_native_keychain_round_trip_and_cleanup() {
    if std::env::var_os(FIXTURE_ROOT_ENV).is_some() {
        private_keychain_child();
        return;
    }
    let root = tempfile::Builder::new()
        .prefix("bboxy-native-fixture.")
        .tempdir()
        .unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let helper = root.path().join("native-fixture");
    std::fs::copy(std::env::current_exe().unwrap(), &helper).unwrap();
    let signed = output(
        Command::new("/usr/bin/codesign")
            .args([
                "--force",
                "--sign",
                "-",
                "--identifier",
                IDENTIFIER,
                "--requirements",
                &format!("=designated => {FIXTURE_REQUIREMENT}"),
            ])
            .arg(&helper),
        "ad-hoc-sign",
    )
    .await;
    assert!(
        signed.status.success(),
        "{}",
        String::from_utf8_lossy(&signed.stderr)
    );
    let result = output(
        Command::new(&helper)
            .args(["--ignored", "--exact", NIGHT, "--nocapture"])
            .env(FIXTURE_ROOT_ENV, root.path()),
        "native-keychain",
    )
    .await;
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("private native fixture completed"));
    let path = root.path().to_owned();
    root.close().unwrap();
    assert!(!path.exists());
}

// The same ignored entry point invokes this only in its signed private child.
fn private_keychain_child() {
    let root =
        PathBuf::from(std::env::var_os(FIXTURE_ROOT_ENV).expect("private fixture root required"));
    assert!(root.is_absolute());
    let meta = std::fs::symlink_metadata(&root).unwrap();
    assert!(meta.is_dir() && !meta.file_type().is_symlink());
    // SAFETY: geteuid has no inputs or side effects.
    assert_eq!(meta.uid(), unsafe { libc::geteuid() });
    assert_eq!(meta.permissions().mode() & 0o077, 0);
    assert!(
        root.file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("bboxy-native-fixture.")
    );
    let root = root.canonicalize().unwrap();
    let _no_ui = SecKeychain::disable_user_interaction().unwrap();
    assert!(!SecKeychain::user_interaction_allowed().unwrap());
    let password = zeroize::Zeroizing::new(ulid::Ulid::new().to_string());
    let private = PrivateKeychain {
        keychain: security_framework::os::macos::keychain::CreateOptions::new()
            .password(&password)
            .prompt_user(false)
            .create(root.join("owned.keychain"))
            .unwrap(),
        deleted: false,
    };
    let service = format!("bboxy450-term.test.{}", ulid::Ulid::new());
    let native = Native {
        service: &service,
        tier: Custody::LoginKeychain,
        fixture: Some(FixtureAccess {
            keychain: &private.keychain,
            requirement: FIXTURE_REQUIREMENT,
        }),
    };
    native.validate_caller().unwrap();
    let mut seed = [0; 32];
    getrandom::fill(&mut seed).unwrap();
    let key = SigningKey::from_bytes(&seed);
    seed.fill(0);
    let expected = key.verifying_key().to_bytes();
    let account = format!("{}.dr1", ulid::Ulid::new());
    assert!(native.read(&account, false).unwrap().is_none());
    native.add(&account, &key).unwrap();
    assert_eq!(native.add(&account, &key), Err(E::JournalConflict));
    assert_eq!(
        native
            .read(&account, false)
            .unwrap()
            .unwrap()
            .verifying_key()
            .to_bytes(),
        expected
    );
    let attributes = native.lookup(&account, false, false).unwrap().unwrap();
    let attributes = attributes.downcast::<CFDictionary>().unwrap();
    // SAFETY: lookup requested Security.framework's attributes dictionary;
    // its keys are CFStrings and its values are retained CF objects. The
    // untyped dictionary downcast above checks its runtime container type.
    let attributes: CFDictionary<CFString, CFType> =
        unsafe { CFDictionary::wrap_under_get_rule(attributes.as_concrete_TypeRef()) };
    // SAFETY: these are the process-lifetime Security.framework attribute keys.
    let service_key = unsafe { CFString::wrap_under_get_rule(kSecAttrService) };
    // SAFETY: same static-key ownership rule as above.
    let account_key = unsafe { CFString::wrap_under_get_rule(kSecAttrAccount) };
    assert_eq!(
        attributes.find(&service_key).unwrap().as_CFType(),
        CFString::new(&service).as_CFType()
    );
    assert_eq!(
        attributes.find(&account_key).unwrap().as_CFType(),
        CFString::new(&account).as_CFType()
    );
    native.remove(&account).unwrap();
    assert!(native.read(&account, false).unwrap().is_none());
    private.delete();
    assert!(!SecKeychain::user_interaction_allowed().unwrap());
    println!(
        "private native fixture completed: created, attributes checked, round-tripped, deleted"
    );
}
