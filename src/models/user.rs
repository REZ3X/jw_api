use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserRow {
    pub id: String,
    pub google_id: String,
    pub username: String,
    pub name: String,
    pub email: String,
    pub avatar_url: Option<String>,
    pub custom_avatar_url: Option<String>,
    pub use_custom_avatar: bool,
    pub bio: Option<String>,
    pub birth: Option<chrono::NaiveDate>,
    pub role: String,
    pub email_verification_status: String,
    pub email_verification_token: Option<String>,
    pub email_verified_at: Option<NaiveDateTime>,
    pub encryption_salt: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub email: String,
    pub exp: i64,
    pub iat: i64,
}


#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub name: String,
    pub email: String,
    pub avatar_url: Option<String>,
    pub use_custom_avatar: bool,
    pub bio: Option<String>,
    pub birth: Option<String>,
    pub role: String,
    pub is_government: bool,
    pub email_verified: bool,
    pub created_at: String,
}

impl From<&UserRow> for UserResponse {
    fn from(u: &UserRow) -> Self {
        let active_avatar = if u.use_custom_avatar {
            u.custom_avatar_url.clone().or(u.avatar_url.clone())
        } else {
            u.avatar_url.clone()
        };

        Self {
            id: u.id.clone(),
            username: u.username.clone(),
            name: u.name.clone(),
            email: u.email.clone(),
            avatar_url: active_avatar,
            use_custom_avatar: u.use_custom_avatar,
            bio: u.bio.clone(),
            birth: u.birth.map(|b| b.to_string()),
            role: u.role.clone(),
            is_government: is_gov_role(&u.role),
            email_verified: u.email_verification_status == "verified",
            created_at: u.created_at.to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PublicUserResponse {
    pub id: String,
    pub username: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub role: String,
    pub is_government: bool,
    pub created_at: String,
}

impl From<&UserRow> for PublicUserResponse {
    fn from(u: &UserRow) -> Self {
        let active_avatar = if u.use_custom_avatar {
            u.custom_avatar_url.clone().or(u.avatar_url.clone())
        } else {
            u.avatar_url.clone()
        };

        Self {
            id: u.id.clone(),
            username: u.username.clone(),
            name: u.name.clone(),
            avatar_url: active_avatar,
            bio: u.bio.clone(),
            role: u.role.clone(),
            is_government: is_gov_role(&u.role),
            created_at: u.created_at.to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
    pub is_new_user: bool,
}


#[derive(Debug, Deserialize)]
pub struct GoogleCallbackRequest {
    pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct GoogleUserInfo {
    pub id: String,
    pub email: String,
    pub name: String,
    pub picture: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct VerifyEmailQuery {
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub name: Option<String>,
    pub bio: Option<String>,
    pub birth: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRoleRequest {
    pub role: String,
}


pub fn is_gov_role(role: &str) -> bool {
    matches!(
        role,
        "city_major_gov" | "fire_department" | "health_department" | "environment_department" | "police_department"
    )
}
