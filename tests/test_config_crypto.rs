use jw_api::crypto::CryptoService;
use jw_api::config::Config;

#[test]
fn test_config_from_env_defaults() {
    std::env::set_var("APP_NAME", "Test App");
    std::env::set_var("JWT_SECRET", "secret");
    std::env::set_var("DATABASE_URL", "mysql://localhost/test");
    std::env::set_var("GOOGLE_CLIENT_ID", "id");
    std::env::set_var("GOOGLE_CLIENT_SECRET", "sec");
    std::env::set_var("GOOGLE_REDIRECT_URI", "uri");
    std::env::set_var("BREVO_SMTP_USER", "user");
    std::env::set_var("BREVO_SMTP_PASS", "pass");
    std::env::set_var("ENCRYPTION_MASTER_KEY", "b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0");
    std::env::set_var("GEMINI_API_KEY", "gemini_key");

    let config = Config::from_env().expect("Failed to parse config");
    
    assert_eq!(config.app.name, "Test App");
    assert_eq!(config.app.env, "development"); // Default
    assert_eq!(config.app.port, 8000); // Default
    assert_eq!(config.app.frontend_url, "http://localhost:3000"); // Default
}

#[test]
fn test_crypto_initialization() {
    let valid_hex = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
    assert!(CryptoService::new(valid_hex).is_ok());

    let invalid_hex = "123";
    assert!(CryptoService::new(invalid_hex).is_err());
}

#[test]
fn test_crypto_encrypt_decrypt_roundtrip() {
    let master_key = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
    let crypto = CryptoService::new(master_key).unwrap();
    
    let salt = CryptoService::generate_user_salt();
    let plaintext = "This is a sensitive message!";
    
    let encrypted = crypto.encrypt(plaintext, &salt).unwrap();
    assert_ne!(plaintext, encrypted);
    
    let decrypted = crypto.decrypt(&encrypted, &salt).unwrap();
    assert_eq!(plaintext, decrypted);
}

#[test]
fn test_crypto_wrong_salt_fails() {
    let master_key = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
    let crypto = CryptoService::new(master_key).unwrap();
    
    let salt1 = CryptoService::generate_user_salt();
    let salt2 = CryptoService::generate_user_salt();
    
    let encrypted = crypto.encrypt("Secret", &salt1).unwrap();
    
    assert!(crypto.decrypt(&encrypted, &salt2).is_err());
}
