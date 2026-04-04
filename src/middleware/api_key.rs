use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use serde_json::json;

use crate::state::AppState;

pub async fn api_key_layer(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, (StatusCode, axum::Json<serde_json::Value>)> {
    if state.config.app.mode != "external" {
        return Ok(next.run(req).await);
    }

    let api_key = req.headers().get("X-API-Key").and_then(|v| v.to_str().ok());

    match (api_key, &state.config.app.api_key) {
        (Some(provided), Some(expected)) if provided == expected => Ok(next.run(req).await),
        _ => Err((
            StatusCode::UNAUTHORIZED,
            axum::Json(json!({
                "success": false,
                "error": "Invalid or missing API key"
            })),
        )),
    }
}
