use sqlx::MySqlPool;
use uuid::Uuid;

use crate::{
    error::{AppError, Result},
    models::{
        PostRow, PostMediaRow, PostTagRow, PostResponse, MediaResponse,
        PostFilterParams, PostListResponse, CreatePostRequest, UpdatePostRequest,
    },
};

pub struct PostService;

impl PostService {
    pub async fn create_post(
        pool: &MySqlPool,
        user_id: &str,
        req: &CreatePostRequest,
        department: &str,
    ) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let is_private = req.is_private.unwrap_or(false);
        let editable_until = chrono::Utc::now().naive_utc() + chrono::Duration::hours(24);

        sqlx::query(
            r#"INSERT INTO posts (id, user_id, caption, location, latitude, longitude, is_private, department, status, editable_until)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'pending', ?)"#,
        )
        .bind(&id)
        .bind(user_id)
        .bind(&req.caption)
        .bind(&req.location)
        .bind(req.latitude)
        .bind(req.longitude)
        .bind(is_private)
        .bind(department)
        .bind(editable_until)
        .execute(pool)
        .await
        .map_err(AppError::DatabaseError)?;


        let tags = Self::extract_tags(&req.caption);
        for tag in &tags {
            let tag_id = Uuid::new_v4().to_string();
            sqlx::query("INSERT IGNORE INTO post_tags (id, post_id, tag) VALUES (?, ?, ?)")
                .bind(&tag_id)
                .bind(&id)
                .bind(tag)
                .execute(pool)
                .await
                .ok();
        }

        Ok(id)
    }

    pub async fn add_media(
        pool: &MySqlPool,
        post_id: &str,
        media_url: &str,
        media_type: &str,
        display_order: i8,
    ) -> Result<()> {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO post_media (id, post_id, media_url, media_type, display_order) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(post_id)
        .bind(media_url)
        .bind(media_type)
        .bind(display_order)
        .execute(pool)
        .await
        .map_err(AppError::DatabaseError)?;
        Ok(())
    }

    pub async fn get_post(
        pool: &MySqlPool,
        post_id: &str,
        viewer_user_id: Option<&str>,
    ) -> Result<PostResponse> {
        let post: PostRow = sqlx::query_as("SELECT * FROM posts WHERE id = ?")
            .bind(post_id)
            .fetch_optional(pool)
            .await
            .map_err(AppError::DatabaseError)?
            .ok_or_else(|| AppError::NotFound("Post not found".into()))?;

        Self::build_post_response(pool, &post, viewer_user_id).await
    }

    pub async fn list_posts(
        pool: &MySqlPool,
        params: &PostFilterParams,
        viewer_user_id: Option<&str>,
        include_private: bool,
    ) -> Result<PostListResponse> {
        let page = params.page.unwrap_or(1).max(1);
        let per_page = params.per_page.unwrap_or(20).min(50);
        let offset = (page - 1) * per_page;

        let mut where_clauses = Vec::new();
        let mut bind_values: Vec<String> = Vec::new();

        if !include_private {
            where_clauses.push("p.is_private = FALSE".to_string());
        }

        let is_filtered = params.department.is_some()
            || params.status.is_some()
            || params.user_id.is_some()
            || params.search.is_some()
            || params.tag.is_some()
            || params.sort.is_some();

        if !is_filtered {
            where_clauses.push("p.created_at >= DATE_SUB(NOW(), INTERVAL 48 HOUR)".to_string());
        }

        if let Some(ref dept) = params.department {
            where_clauses.push("p.department = ?".to_string());
            bind_values.push(dept.clone());
        }

        if let Some(ref status) = params.status {
            where_clauses.push("p.status = ?".to_string());
            bind_values.push(status.clone());
        }

        if let Some(ref uid) = params.user_id {
            where_clauses.push("p.user_id = ?".to_string());
            bind_values.push(uid.clone());
        }

        if let Some(ref search) = params.search {
            where_clauses.push("p.caption LIKE ?".to_string());
            bind_values.push(format!("%{}%", search));
        }

        if let Some(ref tag) = params.tag {
            where_clauses.push("EXISTS (SELECT 1 FROM post_tags pt WHERE pt.post_id = p.id AND pt.tag = ?)".to_string());
            bind_values.push(tag.clone());
        }

        let where_sql = if where_clauses.is_empty() {
            "1=1".to_string()
        } else {
            where_clauses.join(" AND ")
        };

        let order_by = if !is_filtered {
            "RAND()".to_string()
        } else {
            match params.sort.as_deref() {
                Some("most_upvoted") => "p.upvote_count DESC".to_string(),
                Some("most_discussed") => "p.comment_count DESC".to_string(),
                _ => "p.created_at DESC".to_string(),
            }
        };


        let count_sql = format!("SELECT COUNT(*) FROM posts p WHERE {}", where_sql);
        let mut count_query = sqlx::query_as::<_, (i64,)>(&count_sql);
        for val in &bind_values {
            count_query = count_query.bind(val);
        }
        let (total,) = count_query.fetch_one(pool).await.map_err(AppError::DatabaseError)?;


        let fetch_sql = format!(
            "SELECT p.* FROM posts p WHERE {} ORDER BY {} LIMIT ? OFFSET ?",
            where_sql, order_by
        );
        let mut fetch_query = sqlx::query_as::<_, PostRow>(&fetch_sql);
        for val in &bind_values {
            fetch_query = fetch_query.bind(val);
        }
        fetch_query = fetch_query.bind(per_page).bind(offset);

        let rows: Vec<PostRow> = fetch_query.fetch_all(pool).await.map_err(AppError::DatabaseError)?;

        let mut posts = Vec::new();
        for row in &rows {
            let resp = Self::build_post_response(pool, row, viewer_user_id).await?;
            posts.push(resp);
        }

        Ok(PostListResponse { posts, total, page, per_page })
    }

    pub async fn update_post(
        pool: &MySqlPool,
        post_id: &str,
        user_id: &str,
        req: &UpdatePostRequest,
    ) -> Result<()> {
        let post: PostRow = sqlx::query_as("SELECT * FROM posts WHERE id = ? AND user_id = ?")
            .bind(post_id)
            .bind(user_id)
            .fetch_optional(pool)
            .await
            .map_err(AppError::DatabaseError)?
            .ok_or_else(|| AppError::NotFound("Post not found".into()))?;

        let mut updates = Vec::new();
        let mut binds: Vec<String> = Vec::new();

        if let Some(ref caption) = req.caption {
            let now = chrono::Utc::now().naive_utc();
            if now > post.editable_until {
                return Err(AppError::Forbidden("Post content can only be edited within 24 hours".into()));
            }
            updates.push("caption = ?");
            binds.push(caption.clone());
            updates.push("is_edited = TRUE");
        }

        if let Some(ref location) = req.location {
            updates.push("location = ?");
            binds.push(location.clone());
        }

        if let Some(is_private) = req.is_private {

            if post.status == "closed" && !is_private && post.is_private {
                return Err(AppError::Forbidden("Closed private posts cannot be made public".into()));
            }
            updates.push("is_private = ?");
            binds.push(is_private.to_string());
        }

        if updates.is_empty() {
            return Err(AppError::ValidationError("No fields to update".into()));
        }

        updates.push("updated_at = NOW()");
        let sql = format!("UPDATE posts SET {} WHERE id = ?", updates.join(", "));
        let mut q = sqlx::query(&sql);
        for val in &binds {
            if val == "true" || val == "false" {
                q = q.bind(val == "true");
            } else {
                q = q.bind(val);
            }
        }
        q = q.bind(post_id);
        q.execute(pool).await.map_err(AppError::DatabaseError)?;


        if req.caption.is_some() {
            let caption = req.caption.as_ref().unwrap();
            sqlx::query("DELETE FROM post_tags WHERE post_id = ?")
                .bind(post_id)
                .execute(pool)
                .await
                .ok();
            let tags = Self::extract_tags(caption);
            for tag in &tags {
                let tag_id = Uuid::new_v4().to_string();
                sqlx::query("INSERT IGNORE INTO post_tags (id, post_id, tag) VALUES (?, ?, ?)")
                    .bind(&tag_id)
                    .bind(post_id)
                    .bind(tag)
                    .execute(pool)
                    .await
                    .ok();
            }
        }

        Ok(())
    }

    pub async fn delete_post(pool: &MySqlPool, post_id: &str, user_id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM posts WHERE id = ? AND user_id = ?")
            .bind(post_id)
            .bind(user_id)
            .execute(pool)
            .await
            .map_err(AppError::DatabaseError)?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Post not found".into()));
        }
        Ok(())
    }

    pub async fn get_media_count(pool: &MySqlPool, post_id: &str) -> Result<i64> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM post_media WHERE post_id = ?")
            .bind(post_id)
            .fetch_one(pool)
            .await
            .map_err(AppError::DatabaseError)?;
        Ok(count)
    }



    async fn build_post_response(
        pool: &MySqlPool,
        post: &PostRow,
        viewer_user_id: Option<&str>,
    ) -> Result<PostResponse> {

        let user: (String, String, Option<String>, Option<String>, bool, String) = sqlx::query_as(
            "SELECT username, name, avatar_url, custom_avatar_url, use_custom_avatar, role FROM users WHERE id = ?"
        )
        .bind(&post.user_id)
        .fetch_one(pool)
        .await
        .map_err(AppError::DatabaseError)?;

        let avatar = if user.4 {
            user.3.or(user.2.clone())
        } else {
            user.2.clone()
        };


        let media_rows: Vec<PostMediaRow> = sqlx::query_as(
            "SELECT * FROM post_media WHERE post_id = ? ORDER BY display_order"
        )
        .bind(&post.id)
        .fetch_all(pool)
        .await
        .map_err(AppError::DatabaseError)?;

        let media: Vec<MediaResponse> = media_rows
            .iter()
            .map(|m| MediaResponse {
                id: m.id.clone(),
                media_url: m.media_url.clone(),
                media_type: m.media_type.clone(),
                display_order: m.display_order,
            })
            .collect();


        let tag_rows: Vec<PostTagRow> = sqlx::query_as(
            "SELECT * FROM post_tags WHERE post_id = ?"
        )
        .bind(&post.id)
        .fetch_all(pool)
        .await
        .map_err(AppError::DatabaseError)?;

        let tags: Vec<String> = tag_rows.iter().map(|t| t.tag.clone()).collect();


        let my_vote = if let Some(uid) = viewer_user_id {
            let vote: Option<(String,)> = sqlx::query_as(
                "SELECT vote_type FROM post_votes WHERE post_id = ? AND user_id = ?"
            )
            .bind(&post.id)
            .bind(uid)
            .fetch_optional(pool)
            .await
            .map_err(AppError::DatabaseError)?;
            vote.map(|v| v.0)
        } else {
            None
        };

        Ok(PostResponse {
            id: post.id.clone(),
            user_id: post.user_id.clone(),
            username: user.0,
            user_name: user.1,
            user_avatar: avatar,
            user_role: user.5,
            caption: post.caption.clone(),
            location: post.location.clone(),
            latitude: post.latitude,
            longitude: post.longitude,
            is_private: post.is_private,
            department: post.department.clone(),
            status: post.status.clone(),
            upvote_count: post.upvote_count,
            downvote_count: post.downvote_count,
            comment_count: post.comment_count,
            is_edited: post.is_edited,
            media,
            tags,
            my_vote,
            created_at: post.created_at.to_string(),
            updated_at: post.updated_at.to_string(),
        })
    }

    fn extract_tags(caption: &str) -> Vec<String> {
        caption
            .split_whitespace()
            .filter(|word| word.starts_with('#') && word.len() > 1)
            .map(|tag| {
                tag.trim_start_matches('#')
                    .to_lowercase()
                    .chars()
                    .filter(|c| c.is_alphanumeric() || *c == '_')
                    .collect::<String>()
            })
            .filter(|t| !t.is_empty())
            .collect()
    }
}
