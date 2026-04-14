use std::collections::HashSet;
use std::time::Instant;
use jw_api::crypto::CryptoService;

mod common;

const MASTER_KEY: &str = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";

#[test]
fn perf_encrypt_decrypt_throughput() {
    let crypto = CryptoService::new(MASTER_KEY).unwrap();
    let salt = CryptoService::generate_user_salt();
    let count = 500;

    let start = Instant::now();
    for i in 0..count {
        let msg = format!("Civic report #{} about infrastructure damage", i);
        let enc = crypto.encrypt(&msg, &salt).unwrap();
        let dec = crypto.decrypt(&enc, &salt).unwrap();
        assert_eq!(msg, dec);
    }
    let elapsed = start.elapsed();
    common::assert_under("encrypt_decrypt_500x", elapsed, 2000);
}

#[tokio::test]
async fn perf_concurrent_encrypt_decrypt() {
    let crypto = CryptoService::new(MASTER_KEY).unwrap();
    let mut handles = Vec::new();

    let start = Instant::now();
    for i in 0..50u32 {
        let c = crypto.clone();
        handles.push(tokio::spawn(async move {
            let salt = CryptoService::generate_user_salt();
            let msg = format!("Concurrent msg #{}", i);
            let enc = c.encrypt(&msg, &salt).unwrap();
            let dec = c.decrypt(&enc, &salt).unwrap();
            assert_eq!(msg, dec);
        }));
    }

    for h in handles {
        h.await.unwrap();
    }
    let elapsed = start.elapsed();
    common::assert_under("concurrent_50_roundtrips", elapsed, 3000);
}

#[test]
fn perf_large_payload_encryption() {
    let crypto = CryptoService::new(MASTER_KEY).unwrap();
    let salt = CryptoService::generate_user_salt();
    let payload = "A".repeat(1024 * 1024); // 1 MB

    let start = Instant::now();
    let enc = crypto.encrypt(&payload, &salt).unwrap();
    let dec = crypto.decrypt(&enc, &salt).unwrap();
    let elapsed = start.elapsed();

    assert_eq!(payload, dec);
    common::assert_under("1mb_encrypt_decrypt", elapsed, 500);
}

#[test]
fn crypto_truncated_ciphertext_rejected() {
    let crypto = CryptoService::new(MASTER_KEY).unwrap();
    let salt = CryptoService::generate_user_salt();

    let enc = crypto.encrypt("test", &salt).unwrap();
    let truncated = &enc[..8];
    assert!(crypto.decrypt(truncated, &salt).is_err());
}

#[test]
fn crypto_empty_plaintext_roundtrip() {
    let crypto = CryptoService::new(MASTER_KEY).unwrap();
    let salt = CryptoService::generate_user_salt();

    let enc = crypto.encrypt("", &salt).unwrap();
    let dec = crypto.decrypt(&enc, &salt).unwrap();
    assert_eq!(dec, "");
}

#[test]
fn crypto_invalid_base64_rejected() {
    let crypto = CryptoService::new(MASTER_KEY).unwrap();
    let salt = CryptoService::generate_user_salt();
    assert!(crypto.decrypt("not-valid-base64!!!", &salt).is_err());
}

#[test]
fn crypto_wrong_key_length_rejected() {
    assert!(CryptoService::new("abc").is_err());
    assert!(CryptoService::new("").is_err());
    // 31 bytes (62 hex chars) — should fail, need exactly 32
    assert!(CryptoService::new(&"ab".repeat(31)).is_err());
}

#[test]
fn crypto_salt_uniqueness() {
    let mut salts = HashSet::new();
    for _ in 0..1000 {
        let salt = CryptoService::generate_user_salt();
        assert!(salts.insert(salt), "Salt collision detected");
    }
}

#[test]
fn crypto_different_salts_produce_different_ciphertexts() {
    let crypto = CryptoService::new(MASTER_KEY).unwrap();
    let salt_a = CryptoService::generate_user_salt();
    let salt_b = CryptoService::generate_user_salt();
    let msg = "same message";

    let enc_a = crypto.encrypt(msg, &salt_a).unwrap();
    let enc_b = crypto.encrypt(msg, &salt_b).unwrap();
    assert_ne!(enc_a, enc_b);

    assert!(crypto.decrypt(&enc_a, &salt_b).is_err());
}
