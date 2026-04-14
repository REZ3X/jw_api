use std::time::Instant;
use jw_api::services::GeminiService;

mod common;

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
async fn perf_generate_text_response_time() {
    let (api_key, model) = load_env_and_get_model();
    let service = GeminiService::new(api_key, model);
    let mut durations = Vec::new();

    for i in 0..3 {
        let start = Instant::now();
        let result = service.generate_text("Reply with 'ok'").await;
        let elapsed = start.elapsed();
        assert!(result.is_ok(), "Call {} failed: {:?}", i, result.err());
        durations.push(elapsed);
        println!("⏱ generate_text attempt {}: {}ms", i, elapsed.as_millis());
    }

    durations.sort();
    let p95 = durations[(durations.len() as f64 * 0.95) as usize];
    common::assert_under("gemini_text_p95", p95, 15000);
}

#[tokio::test]
#[ignore]
async fn perf_classification_consistency() {
    let (api_key, model) = load_env_and_get_model();
    let service = GeminiService::new(api_key, model);

    let test_cases = vec![
        ("Trash is piling up near the river and causing pollution", "environment_department"),
        ("There is a fire in the building on Jalan Malioboro", "fire_department"),
        ("Criminal activity and theft reported near the market", "police_department"),
    ];

    for (prompt, expected) in &test_cases {
        let mut correct = 0;
        let runs = 5;
        for _ in 0..runs {
            if let Ok(dept) = service.classify_department(prompt).await {
                if dept == *expected {
                    correct += 1;
                }
            }
        }
        println!(
            "⏱ classify '{}...' → {}/{} correct for {}",
            &prompt[..40.min(prompt.len())], correct, runs, expected
        );
        assert!(
            correct >= 4,
            "Classification for '{}' only matched {}/{} times (expected {})",
            prompt, correct, runs, expected
        );
    }
}

#[tokio::test]
#[ignore]
async fn perf_title_generation_invariants() {
    let (api_key, model) = load_env_and_get_model();
    let service = GeminiService::new(api_key, model);

    let prompts = vec![
        "There is a huge pothole on Jalan Sudirman causing accidents",
        "My neighbor's house caught fire and we need the fire department",
        "Illegal dumping in the river near Tugu station",
        "Dog attacked people in the park area near the mosque",
        "Hospital refuse to treat emergency patient without insurance",
    ];

    for prompt in &prompts {
        let title = service.generate_chat_title(prompt).await.unwrap();
        let word_count = title.split_whitespace().count();
        let char_count = title.chars().count();

        println!("⏱ title for '{}...' → '{}' ({} words, {} chars)", &prompt[..30], title, word_count, char_count);

        assert!(!title.is_empty(), "Title should not be empty");
        assert!(word_count <= 7, "Title '{}' exceeds 7 words (got {})", title, word_count);
        assert!(char_count <= 60, "Title '{}' exceeds 60 chars (got {})", title, char_count);
    }
}

#[tokio::test]
#[ignore]
async fn perf_concurrent_classification() {
    let (api_key, model) = load_env_and_get_model();
    let service = GeminiService::new(api_key, model);

    let valid_departments = vec![
        "city_major_gov", "fire_department", "health_department",
        "environment_department", "police_department",
    ];

    let prompts = vec![
        "Broken traffic light causing congestion",
        "Chemical spill in the river",
        "Hospital beds unavailable for patients",
    ];

    let start = Instant::now();
    let mut handles = Vec::new();
    for prompt in prompts {
        let svc = service.clone();
        handles.push(tokio::spawn(async move {
            svc.classify_department(prompt).await
        }));
    }

    for h in handles {
        let result = h.await.unwrap();
        assert!(result.is_ok(), "Concurrent classification failed: {:?}", result.err());
        let dept = result.unwrap();
        assert!(
            valid_departments.contains(&dept.as_str()),
            "Got invalid department: {}", dept
        );
    }
    let elapsed = start.elapsed();
    common::assert_under("concurrent_3_classifications", elapsed, 30000);
}

#[tokio::test]
#[ignore]
async fn gemini_empty_prompt_handled() {
    let (api_key, model) = load_env_and_get_model();
    let service = GeminiService::new(api_key, model);

    let result = service.generate_text("").await;
    // Should either succeed with some response or return a clean error
    match result {
        Ok(text) => println!("⏱ empty prompt returned: '{}'", &text[..50.min(text.len())]),
        Err(e) => println!("⏱ empty prompt error (acceptable): {}", e),
    }
}

#[tokio::test]
#[ignore]
async fn gemini_long_prompt_handled() {
    let (api_key, model) = load_env_and_get_model();
    let service = GeminiService::new(api_key, model);

    let long_prompt = "Report about civic issues. ".repeat(100); // ~2700 chars

    let start = Instant::now();
    let result = service.generate_text(&long_prompt).await;
    let elapsed = start.elapsed();

    assert!(result.is_ok(), "Long prompt should not panic: {:?}", result.err());
    println!("⏱ long_prompt ({}chars): {}ms", long_prompt.len(), elapsed.as_millis());
}

#[tokio::test]
#[ignore]
async fn gemini_agent_tool_calling_format() {
    let (api_key, model) = load_env_and_get_model();
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
        println!("⏱ tool_call '{}' → contains '{}'", user_msg, expected_tool);
        assert!(
            resp.contains(expected_tool),
            "Response didn't trigger expected tool {}. Raw: {}", expected_tool, resp
        );
    }
}
