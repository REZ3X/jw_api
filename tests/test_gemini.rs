use jw_api::services::GeminiService;

#[tokio::test]
#[ignore]
async fn test_live_gemini_generation() {
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let _ = dotenvy::from_path(std::path::Path::new(&manifest_dir).join(".env"));
    } else {
        let _ = dotenvy::dotenv();
    }
    let api_key = std::env::var("GEMINI_API_KEY").expect("Need GEMINI_API_KEY for live tests");
    let model = std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-1.5-flash".to_string());
    let service = GeminiService::new(api_key, model);
    
    let result = service.generate_text("Say 'test'").await;
    assert!(result.is_ok(), "Gemini call failed: {:?}", result.err());
}

#[tokio::test]
#[ignore]
async fn test_live_gemini_department_classification() {
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let _ = dotenvy::from_path(std::path::Path::new(&manifest_dir).join(".env"));
    } else {
        let _ = dotenvy::dotenv();
    }
    let api_key = std::env::var("GEMINI_API_KEY").expect("Need GEMINI_API_KEY for live tests");
    let model = std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-1.5-flash".to_string());
    let service = GeminiService::new(api_key, model);
    
    let trash_prompt = "There is trash everywhere in my neighborhood.";
    let dept = service.classify_department(trash_prompt).await.unwrap();
    assert_eq!(dept, "environment_department");
}
