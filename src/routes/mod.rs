pub mod health;
pub mod auth;
pub mod user;
pub mod post;
pub mod comment;
pub mod vote;
pub mod department;
pub mod chat;
pub mod analytics;
pub mod log;
pub mod dev;

use axum::{Router, extract::DefaultBodyLimit};
use tower_http::services::ServeDir;

use crate::state::AppState;

pub fn build_router(state: AppState) -> Router {
    let upload_dir = state.config.media.upload_dir.clone();

    // Set max body size to roughly 250MB to accommodate large file uploads,
    // custom logic in routes will handle specific size limits per file type.
    let body_limit = DefaultBodyLimit::max(250 * 1024 * 1024);

    Router::new()
        .nest("/api/auth", auth::routes())
        .nest("/api/users", user::routes())
        .nest("/api/posts", post::routes())
        .nest("/api/comments", comment::routes())
        .nest("/api/votes", vote::routes())
        .nest("/api/departments", department::routes())
        .nest("/api/chats", chat::routes())
        .nest("/api/analytics", analytics::routes())
        .nest("/api/logs", log::routes())
        .nest("/api/dev", dev::routes())
        .merge(health::routes())
        .nest_service("/uploads", ServeDir::new(&upload_dir))
        .layer(body_limit)
        .with_state(state)
}
