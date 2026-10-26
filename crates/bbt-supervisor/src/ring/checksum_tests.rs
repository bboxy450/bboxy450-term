use super::*;

// keeps: spec mux-daemon.md — Ring records, checkpoints and eviction
#[test]
fn crc_matches_the_table_for_all_bytes_unaligned_splits_and_short_tails() {
    assert_eq!(crc32c(b""), 0);
    assert_eq!(crc32c(b"123456789"), 0xe3069283);
    assert_eq!(crc32c_parts(b"1234", b"56789"), 0xe3069283);
    for byte in 0..=255 {
        assert_eq!(crc32c(&[byte]), crc32c_portable(&[byte], &[]));
    }
    let bytes: Vec<u8> = (0..8192).map(|n| n as u8).collect();
    for offset in 0..16 {
        for tail in 0..16 {
            for split in [0, 1, 7, 8, 15, 16, 31, 32] {
                let header = &bytes[offset..offset + split];
                let payload = &bytes[15 - offset..15 - offset + 64 + tail];
                assert_eq!(
                    crc32c_parts(header, payload),
                    crc32c_portable(header, payload)
                );
            }
        }
    }
    let mut state = 0x37d9_1e60_7143_28a5u64;
    for _ in 0..2048 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let header_start = (state >> 40) as usize & 15;
        let payload_start = (state >> 44) as usize & 15;
        let header_len = state as usize % 257;
        let payload_len = (state >> 16) as usize % 4097;
        let header = &bytes[header_start..header_start + header_len];
        let payload = &bytes[payload_start..payload_start + payload_len];
        assert_eq!(
            crc32c_parts(header, payload),
            crc32c_portable(header, payload)
        );
    }
    #[cfg(target_arch = "aarch64")]
    println!(
        "checksum CPU features: crc={} sha2={}",
        std::arch::is_aarch64_feature_detected!("crc"),
        std::arch::is_aarch64_feature_detected!("sha2")
    );
}

// keeps: spec mux-daemon.md — Ring records, checkpoints and eviction
#[test]
#[allow(clippy::disallowed_methods)] // Copy one pre-change synthetic blob into an owned tempdir.
fn fixed_sha_vectors_and_a_software_written_cold_blob_still_verify() {
    use crate::store::{Blobs, digest};
    assert_eq!(
        digest(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        digest(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let bytes: Vec<u8> = (0..1024 * 1024).map(|n| n as u8).collect();
    let old_digest = "fbbab289f7f94b25736c58be46a994c441fd02552cc6022352e3d86d2fab7c83";
    assert_eq!(digest(&bytes), old_digest);
    // Produced by software sha2 at 416eff6, before enabling asm-aarch64.
    // Generator, source hash and exact command: w1-2f-legacy-blob-*.log.
    let fixture = include_bytes!("../../tests/fixtures/legacy-checksum.zst");
    let temp = tempfile::tempdir().unwrap();
    let blobs = LocalBlobs::open(temp.path()).unwrap();
    let path = temp.path().join("blobs").join(format!("{old_digest}.zst"));
    std::fs::write(&path, fixture).unwrap();
    assert_eq!(blobs.get(old_digest).unwrap(), bytes);
    assert_eq!(blobs.put(&bytes).unwrap(), old_digest);
    assert_eq!(std::fs::read(path).unwrap(), fixture);
}
