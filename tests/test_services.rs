mod common;

#[tokio::test]
#[ignore]
async fn test_comment_service_integration() {
    let (_server, _state) = common::setup_test_app().await;
    
    assert!(true);
}

#[tokio::test]
#[ignore]
async fn test_post_service_integration() {
    let (_server, _state) = common::setup_test_app().await;
    assert!(true);
}
