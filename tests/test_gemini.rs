use jw_api::services::GeminiService;

fn load_env_and_get_model() -> (String, String) {
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let _ = dotenvy::from_path(std::path::Path::new(&manifest_dir).join(".env"));
    } else {
        let _ = dotenvy::dotenv();
    }
    let api_key = std::env::var("GEMINI_API_KEY").expect("Need GEMINI_API_KEY for live tests");
    let model = std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-2.5-flash".to_string());
    (api_key, model)
}

#[tokio::test]
#[ignore]
async fn test_live_gemini_generation() {
    let (api_key, model) = load_env_and_get_model();
    println!("➤ Running test_live_gemini_generation using Gemini model: {}", model);
    let service = GeminiService::new(api_key, model);
    
    let result = service.generate_text("Say 'test'").await;
    assert!(result.is_ok(), "Gemini call failed: {:?}", result.err());
    println!("  └─ Success: Received response.");
}

#[tokio::test]
#[ignore]
async fn test_live_gemini_department_classification() {
    let (api_key, model) = load_env_and_get_model();
    println!("➤ Running test_live_gemini_department_classification using Gemini model: {}", model);
    let service = GeminiService::new(api_key, model);
    
    let trash_prompt = "There is trash everywhere in my neighborhood.";
    let dept = service.classify_department(trash_prompt).await.unwrap();
    println!("  └─ Classified as: {}", dept);
    assert_eq!(dept, "environment_department");
}

#[tokio::test]
#[ignore]
async fn test_live_gemini_chat_title_generation() {
    let (api_key, model) = load_env_and_get_model();
    println!("➤ Running test_live_gemini_chat_title_generation using Gemini model: {}", model);
    let service = GeminiService::new(api_key, model);
    
    let prompt = "Hello, I need to report a broken traffic light on Main St causing chaos.";
    let title = service.generate_chat_title(prompt).await.unwrap();
    println!("  └─ Generated Title: '{}'", title);
    
    assert!(!title.is_empty());
    assert!(title.split_whitespace().count() <= 7); // Usually around 3-5 words
}

#[tokio::test]
#[ignore]
async fn test_live_gemini_agent_tool_calling_format() {
    let (api_key, model) = load_env_and_get_model();
    println!("➤ Running test_live_gemini_agent_tool_calling_format using Gemini model: {}", model);
    let service = GeminiService::new(api_key, model);
    
    let system_prompt = r#"You are JW AI, a civic engagement assistant for the JogjaWaskita platform.
User: Test User (Role: Citizen)
SHARED TOOLS:
- GET_TRENDING_TAGS
- GET_PLATFORM_STATS
- SEARCH_POSTS

CITIZEN TOOLS:
- GET_MY_POSTS
- GET_MY_UNRESPONDED_POSTS
- CREATE_POST_DRAFT

RESPONSE FORMAT (when calling tools):
{"response": "Brief context", "tool_calls": [{"tool_name": "TOOL_NAME", "parameters": {}}]}
"#;

    let prompts = vec![
        ("Show me trending platform tags", "GET_TRENDING_TAGS"),
        ("What are my unresponded posts?", "GET_MY_UNRESPONDED_POSTS"),
        ("I want to draft a report about a fire on 5th Ave", "CREATE_POST_DRAFT"),
    ];

    for (user_msg, expected_tool) in prompts {
        let resp = service.generate_chat_response(system_prompt, &[], user_msg, 0.0).await.unwrap();
        println!("  └─ Prompt: '{}' -> Triggered Tool string payload containing: {}", user_msg, expected_tool);
        assert!(
            resp.contains(expected_tool), 
            "Response didn't trigger expected tool {}. Raw response: {}", expected_tool, resp
        );
    }
}
