use base64::{engine::general_purpose::STANDARD, Engine as _};
use jw_api::error::AppError;
use jw_api::models::{CreateCommentRequest, CreatePostRequest};
use jw_api::services::{CommentService, MediaService, PostService, VoteService};

mod common;

#[tokio::test]
#[ignore]
async fn post_create_fetch_verify() {
    let (_, state) = common::setup_test_app().await;
    jw_api::db::run_migrations(&state.db).await.unwrap();

    let (user_id, _salt) = common::create_test_user(&state.db, "svc_post_create").await;

    let (elapsed, post_id) = common::timed("post_create", async {
        let req = CreatePostRequest {
            caption: "Broken road on Jalan Kaliurang #infrastructure #urgent".into(),
            location: Some("Kaliurang KM 9".into()),
            latitude: Some(-7.75),
            longitude: Some(110.38),
            is_private: Some(false),
            department: None,
        };
        PostService::create_post(&state.db, &user_id, &req, "city_major_gov")
            .await
            .unwrap()
    })
    .await;
    common::assert_under("post_create", elapsed, 2000);

    let post = PostService::get_post(&state.db, &post_id, Some(&user_id))
        .await
        .unwrap();
    assert_eq!(
        post.caption,
        "Broken road on Jalan Kaliurang #infrastructure #urgent"
    );
    assert_eq!(post.department, "city_major_gov");
    assert_eq!(post.status, "pending");
    assert!(!post.is_private);
    assert!(post.tags.contains(&"infrastructure".to_string()));
    assert!(post.tags.contains(&"urgent".to_string()));

    common::cleanup_test_post(&state.db, &post_id).await;
    common::cleanup_test_user(&state.db, &user_id).await;
}

#[tokio::test]
#[ignore]
async fn post_delete_returns_404() {
    let (_, state) = common::setup_test_app().await;
    jw_api::db::run_migrations(&state.db).await.unwrap();

    let (user_id, _) = common::create_test_user(&state.db, "svc_post_del").await;
    let post_id = common::create_test_post(&state.db, &user_id, "To be deleted #test").await;

    PostService::delete_post(&state.db, &post_id, &user_id)
        .await
        .unwrap();

    let result = PostService::get_post(&state.db, &post_id, None).await;
    assert!(result.is_err());
    match result {
        Err(AppError::NotFound(_)) => {}
        _ => panic!("Expected NotFound after deletion"),
    }

    common::cleanup_test_user(&state.db, &user_id).await;
}

#[tokio::test]
#[ignore]
async fn comment_lifecycle_count_tracking() {
    let (_, state) = common::setup_test_app().await;
    jw_api::db::run_migrations(&state.db).await.unwrap();

    let (user_id, _) = common::create_test_user(&state.db, "svc_comment").await;
    let post_id = common::create_test_post(&state.db, &user_id, "Comment test post").await;

    let req = CreateCommentRequest {
        content: "First comment on this post".into(),
    };
    let comment = CommentService::create_comment(&state.db, &post_id, &user_id, &req, false, None)
        .await
        .unwrap();
    assert_eq!(comment.content, "First comment on this post");
    assert!(!comment.is_official);

    let post = PostService::get_post(&state.db, &post_id, None)
        .await
        .unwrap();
    assert_eq!(
        post.comment_count, 1,
        "Comment count should be 1 after adding a comment"
    );

    CommentService::delete_comment(&state.db, &comment.id, &user_id)
        .await
        .unwrap();

    let post = PostService::get_post(&state.db, &post_id, None)
        .await
        .unwrap();
    assert_eq!(
        post.comment_count, 0,
        "Comment count should be 0 after deleting the comment"
    );

    common::cleanup_test_post(&state.db, &post_id).await;
    common::cleanup_test_user(&state.db, &user_id).await;
}

#[tokio::test]
#[ignore]
async fn vote_toggle_and_switch() {
    let (_, state) = common::setup_test_app().await;
    jw_api::db::run_migrations(&state.db).await.unwrap();

    let (user_id, _) = common::create_test_user(&state.db, "svc_vote").await;
    let post_id = common::create_test_post(&state.db, &user_id, "Vote test post").await;

    // upvote
    let v1 = VoteService::vote_post(&state.db, &post_id, &user_id, "up")
        .await
        .unwrap();
    assert!(v1.voted);
    assert_eq!(v1.vote_type, Some("up".into()));
    assert_eq!(v1.upvote_count, 1);
    assert_eq!(v1.downvote_count, 0);

    // toggle off (same vote again)
    let v2 = VoteService::vote_post(&state.db, &post_id, &user_id, "up")
        .await
        .unwrap();
    assert!(!v2.voted);
    assert_eq!(v2.vote_type, None);
    assert_eq!(v2.upvote_count, 0);

    // switch to downvote
    let v3 = VoteService::vote_post(&state.db, &post_id, &user_id, "up")
        .await
        .unwrap();
    assert!(v3.voted);
    let v4 = VoteService::vote_post(&state.db, &post_id, &user_id, "down")
        .await
        .unwrap();
    assert!(v4.voted);
    assert_eq!(v4.vote_type, Some("down".into()));
    assert_eq!(v4.upvote_count, 0);
    assert_eq!(v4.downvote_count, 1);

    common::cleanup_test_post(&state.db, &post_id).await;
    common::cleanup_test_user(&state.db, &user_id).await;
}

#[tokio::test]
#[ignore]
async fn vote_invalid_type_rejected() {
    let (_, state) = common::setup_test_app().await;
    jw_api::db::run_migrations(&state.db).await.unwrap();

    let (user_id, _) = common::create_test_user(&state.db, "svc_vote_bad").await;
    let post_id = common::create_test_post(&state.db, &user_id, "Bad vote test").await;

    let result = VoteService::vote_post(&state.db, &post_id, &user_id, "sideways").await;
    assert!(result.is_err());
    match result {
        Err(AppError::ValidationError(msg)) => {
            assert!(msg.contains("up") || msg.contains("down"));
        }
        _ => panic!("Expected ValidationError for invalid vote type"),
    }

    common::cleanup_test_post(&state.db, &post_id).await;
    common::cleanup_test_user(&state.db, &user_id).await;
}

#[tokio::test]
async fn test_media_upload_size_limit() {
    let upload_dir = "./tests_temp_uploads";
    let _ = tokio::fs::create_dir_all(upload_dir).await;

    let max_size = 2 * 1024 * 1024;
    let under_limit_base64 = STANDARD.encode(vec![0u8; 1 * 1024 * 1024]);
    let under_limit_bytes = STANDARD.decode(&under_limit_base64).unwrap();

    let result = MediaService::save_file(
        upload_dir,
        "posts",
        "under_limit.jpg",
        &under_limit_bytes,
        max_size,
    )
    .await;
    assert!(
        result.is_ok(),
        "File under limit should be uploaded successfully"
    );
    let url = result.unwrap();
    assert!(url.starts_with("/uploads/posts/"));
    let _ = MediaService::delete_file(upload_dir, &url).await;

    let over_limit_base64 = STANDARD.encode(vec![0u8; 3 * 1024 * 1024]);
    let over_limit_bytes = STANDARD.decode(&over_limit_base64).unwrap();

    let result = MediaService::save_file(
        upload_dir,
        "posts",
        "over_limit.jpg",
        &over_limit_bytes,
        max_size,
    )
    .await;
    assert!(result.is_err(), "File over limit should fail to upload");
    match result {
        Err(AppError::PayloadTooLarge(msg)) => {
            assert!(msg.contains("File exceeds max size"));
        }
        _ => panic!("Expected PayloadTooLarge error"),
    }

    let _ = tokio::fs::remove_dir_all(upload_dir).await;
}

#[tokio::test]
#[ignore]
async fn perf_multiple_post_creates() {
    let (_, state) = common::setup_test_app().await;
    jw_api::db::run_migrations(&state.db).await.unwrap();
    let (user_id, _) = common::create_test_user(&state.db, "svc_bulk").await;

    let mut post_ids = Vec::new();
    let (elapsed, _) = common::timed("create_10_posts", async {
        for i in 0..10 {
            let req = CreatePostRequest {
                caption: format!("Bulk test post #{} #loadtest", i),
                location: Some("Test Location".into()),
                latitude: None,
                longitude: None,
                is_private: Some(false),
                department: None,
            };
            let id = PostService::create_post(&state.db, &user_id, &req, "city_major_gov")
                .await
                .unwrap();
            post_ids.push(id);
        }
    })
    .await;

    common::assert_under("bulk_10_post_creates", elapsed, 5000);

    for pid in &post_ids {
        common::cleanup_test_post(&state.db, pid).await;
    }
    common::cleanup_test_user(&state.db, &user_id).await;
}
