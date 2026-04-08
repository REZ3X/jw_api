use jw_api::config::BrevoConfig;
use jw_api::services::EmailService;

#[tokio::test]
#[ignore]
async fn test_email_live_send_verification() {
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let _ = dotenvy::from_path(std::path::Path::new(&manifest_dir).join(".env"));
    } else {
        let _ = dotenvy::dotenv();
    }
    // Requires live brevo credentials in env
    let config = BrevoConfig {
        smtp_host: std::env::var("BREVO_SMTP_HOST").unwrap_or_else(|_| "smtp-relay.brevo.com".to_string()),
        smtp_port: std::env::var("BREVO_SMTP_PORT").unwrap_or_else(|_| "587".to_string()).parse().unwrap(),
        smtp_user: std::env::var("BREVO_SMTP_USER").expect("Needs BREVO_SMTP_USER"),
        smtp_pass: std::env::var("BREVO_SMTP_PASS").expect("Needs BREVO_SMTP_PASS"),
        from_email: std::env::var("BREVO_FROM_EMAIL").unwrap_or_else(|_| "noreply@localhost".to_string()),
        from_name: std::env::var("BREVO_FROM_NAME").unwrap_or_else(|_| "JogjaWaskita".to_string()),
    };
    
    let service = EmailService::new(&config, "JogjaWaskita Test", "http://localhost:3000");
    
    let recipient = std::env::var("TEST_EMAIL_RECIPIENT").expect("Needs TEST_EMAIL_RECIPIENT");
    
    let result = service.send_verification_email(&recipient, "Test User", "dummy-1234").await;
    assert!(result.is_ok(), "Email send failed: {:?}", result.err());
}
