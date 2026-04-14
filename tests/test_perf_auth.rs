use std::collections::HashSet;
use std::time::Instant;
use jw_api::config::Config;
use jw_api::models::{Claims, UserRow, UserResponse, PublicUserResponse};
use jw_api::services::AuthService;

mod common;

fn load_config() -> Config {
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let _ = dotenvy::from_path(std::path::Path::new(&manifest_dir).join(".env"));
    } else {
        let _ = dotenvy::dotenv();
    }
    Config::from_env().unwrap()
}

fn make_test_user(role: &str, verified: bool) -> UserRow {
    UserRow {
        id: "test-id".into(),
        google_id: "g-123".into(),
        username: "testuser".into(),
        name: "Test User".into(),
        email: "test@test.local".into(),
        avatar_url: Some("https://google.com/avatar.jpg".into()),
        custom_avatar_url: None,
        use_custom_avatar: false,
        bio: Some("Test bio".into()),
        birth: None,
        role: role.into(),
        email_verification_status: if verified { "verified" } else { "pending" }.into(),
        email_verification_token: None,
        email_verified_at: None,
        encryption_salt: "abcd1234".into(),
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
    }
}

#[test]
fn perf_jwt_generation_throughput() {
    let config = load_config();
    let user = make_test_user("basic", true);

    let start = Instant::now();
    for _ in 0..100 {
        let token = AuthService::generate_jwt(&user, &config).unwrap();
        assert!(!token.is_empty());
    }
    let elapsed = start.elapsed();
    common::assert_under("jwt_100_tokens", elapsed, 500);
}

#[test]
fn jwt_claims_content_valid() {
    let config = load_config();
    let user = make_test_user("basic", true);

    let token = AuthService::generate_jwt(&user, &config).unwrap();

    let decoded = jsonwebtoken::decode::<Claims>(
        &token,
        &jsonwebtoken::DecodingKey::from_secret(config.jwt.secret.as_bytes()),
        &jsonwebtoken::Validation::default(),
    )
    .unwrap();

    assert_eq!(decoded.claims.sub, "test-id");
    assert_eq!(decoded.claims.email, "test@test.local");
    assert!(decoded.claims.exp > decoded.claims.iat);
    assert!(decoded.claims.exp - decoded.claims.iat == config.jwt.expiration_hours * 3600);
}

#[test]
fn jwt_invalid_secret_rejected() {
    let config = load_config();
    let user = make_test_user("basic", true);
    let token = AuthService::generate_jwt(&user, &config).unwrap();

    let result = jsonwebtoken::decode::<Claims>(
        &token,
        &jsonwebtoken::DecodingKey::from_secret(b"wrong-secret"),
        &jsonwebtoken::Validation::default(),
    );
    assert!(result.is_err());
}

#[test]
fn verification_token_uniqueness() {
    let mut tokens = HashSet::new();
    for _ in 0..500 {
        let token = AuthService::generate_verification_token();
        assert!(tokens.insert(token), "Token collision detected");
    }
}

#[test]
fn verification_token_format() {
    for _ in 0..50 {
        let token = AuthService::generate_verification_token();
        assert_eq!(token.len(), 48, "Token length should be 48, got {}", token.len());
        assert!(
            token.chars().all(|c| c.is_alphanumeric()),
            "Token contains non-alphanumeric char: {}", token
        );
    }
}

#[test]
fn user_response_gov_role_detection() {
    let gov_roles = ["city_major_gov", "fire_department", "health_department", "environment_department", "police_department"];
    let non_gov = ["basic", "dev"];

    for role in gov_roles {
        let user = make_test_user(role, true);
        let resp = UserResponse::from(&user);
        assert!(resp.is_government, "{} should be detected as government", role);
    }
    for role in non_gov {
        let user = make_test_user(role, true);
        let resp = UserResponse::from(&user);
        assert!(!resp.is_government, "{} should NOT be government", role);
    }
}

#[test]
fn user_response_avatar_fallback_logic() {
    // google avatar only
    let mut user = make_test_user("basic", true);
    let resp = UserResponse::from(&user);
    assert_eq!(resp.avatar_url, Some("https://google.com/avatar.jpg".into()));

    // custom avatar active
    user.custom_avatar_url = Some("/uploads/avatars/custom.jpg".into());
    user.use_custom_avatar = true;
    let resp = UserResponse::from(&user);
    assert_eq!(resp.avatar_url, Some("/uploads/avatars/custom.jpg".into()));

    // custom avatar set but not active
    user.use_custom_avatar = false;
    let resp = UserResponse::from(&user);
    assert_eq!(resp.avatar_url, Some("https://google.com/avatar.jpg".into()));

    // no avatar at all
    user.avatar_url = None;
    user.custom_avatar_url = None;
    user.use_custom_avatar = false;
    let resp = UserResponse::from(&user);
    assert_eq!(resp.avatar_url, None);
}

#[test]
fn user_response_email_verification_mapping() {
    let verified = make_test_user("basic", true);
    assert!(UserResponse::from(&verified).email_verified);

    let pending = make_test_user("basic", false);
    assert!(!UserResponse::from(&pending).email_verified);
}

#[test]
fn public_user_response_hides_email() {
    let user = make_test_user("basic", true);
    let public = PublicUserResponse::from(&user);
    // PublicUserResponse should not expose email (struct has no email field)
    let serialized = serde_json::to_value(&public).unwrap();
    assert!(serialized.get("email").is_none(), "Public profile should not expose email");
}
