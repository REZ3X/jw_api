-- JW (JogjaWaskita) — Civic Engagement & Government Report Platform
-- Initial Database Schema for MariaDB
-- Run: mysql -u root -p jw_db < migrations/001_initial_schema.sql

SET NAMES utf8mb4;
SET CHARACTER SET utf8mb4;

-- ============================================================
-- USERS
-- ============================================================
CREATE TABLE IF NOT EXISTS users (
    id CHAR(36) NOT NULL PRIMARY KEY,
    google_id VARCHAR(255) NOT NULL UNIQUE,
    username VARCHAR(255) NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL UNIQUE,
    avatar_url TEXT,
    custom_avatar_url TEXT,
    use_custom_avatar BOOLEAN NOT NULL DEFAULT FALSE,
    bio TEXT,
    birth DATE,
    role ENUM(
        'basic',
        'city_major_gov',
        'fire_department',
        'health_department',
        'environment_department',
        'police_department',
        'dev'
    ) NOT NULL DEFAULT 'basic',
    email_verification_status ENUM('pending', 'verified') NOT NULL DEFAULT 'pending',
    email_verification_token VARCHAR(255),
    email_verified_at DATETIME,
    encryption_salt CHAR(32) NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    INDEX idx_users_email (email),
    INDEX idx_users_google (google_id),
    INDEX idx_users_role (role),
    INDEX idx_users_username (username)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- POSTS (Reports / Aduan)
-- ============================================================
CREATE TABLE IF NOT EXISTS posts (
    id CHAR(36) NOT NULL PRIMARY KEY,
    user_id CHAR(36) NOT NULL,
    caption TEXT NOT NULL,
    location VARCHAR(500),
    latitude DOUBLE,
    longitude DOUBLE,
    is_private BOOLEAN NOT NULL DEFAULT FALSE,
    department ENUM(
        'city_major_gov',
        'fire_department',
        'health_department',
        'environment_department',
        'police_department'
    ) NOT NULL,
    status ENUM('pending', 'in_progress', 'closed') NOT NULL DEFAULT 'pending',
    upvote_count INT NOT NULL DEFAULT 0,
    downvote_count INT NOT NULL DEFAULT 0,
    comment_count INT NOT NULL DEFAULT 0,
    is_edited BOOLEAN NOT NULL DEFAULT FALSE,
    editable_until DATETIME NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    INDEX idx_posts_user (user_id),
    INDEX idx_posts_department (department),
    INDEX idx_posts_status (status),
    INDEX idx_posts_created (created_at DESC),
    INDEX idx_posts_upvotes (upvote_count DESC),
    INDEX idx_posts_private (is_private, department)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- POST MEDIA (Images & Videos)
-- ============================================================
CREATE TABLE IF NOT EXISTS post_media (
    id CHAR(36) NOT NULL PRIMARY KEY,
    post_id CHAR(36) NOT NULL,
    media_url VARCHAR(500) NOT NULL,
    media_type ENUM('image', 'video') NOT NULL DEFAULT 'image',
    display_order TINYINT NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE,
    INDEX idx_media_post (post_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- POST TAGS
-- ============================================================
CREATE TABLE IF NOT EXISTS post_tags (
    id CHAR(36) NOT NULL PRIMARY KEY,
    post_id CHAR(36) NOT NULL,
    tag VARCHAR(100) NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE,
    INDEX idx_tags_post (post_id),
    INDEX idx_tags_tag (tag),
    UNIQUE KEY uq_post_tag (post_id, tag)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- COMMENTS
-- ============================================================
CREATE TABLE IF NOT EXISTS comments (
    id CHAR(36) NOT NULL PRIMARY KEY,
    post_id CHAR(36) NOT NULL,
    user_id CHAR(36) NOT NULL,
    content TEXT NOT NULL,
    is_edited BOOLEAN NOT NULL DEFAULT FALSE,
    is_pinned BOOLEAN NOT NULL DEFAULT FALSE,
    is_official BOOLEAN NOT NULL DEFAULT FALSE,
    official_image_url VARCHAR(500),
    upvote_count INT NOT NULL DEFAULT 0,
    downvote_count INT NOT NULL DEFAULT 0,
    reply_count INT NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    INDEX idx_comments_post (post_id, created_at DESC),
    INDEX idx_comments_user (user_id),
    INDEX idx_comments_upvotes (post_id, upvote_count DESC),
    INDEX idx_comments_pinned (post_id, is_pinned DESC)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- SUB COMMENTS (Replies to Comments)
-- ============================================================
CREATE TABLE IF NOT EXISTS sub_comments (
    id CHAR(36) NOT NULL PRIMARY KEY,
    comment_id CHAR(36) NOT NULL,
    user_id CHAR(36) NOT NULL,
    reply_to_user_id CHAR(36),
    content TEXT NOT NULL,
    is_edited BOOLEAN NOT NULL DEFAULT FALSE,
    is_official BOOLEAN NOT NULL DEFAULT FALSE,
    upvote_count INT NOT NULL DEFAULT 0,
    downvote_count INT NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (comment_id) REFERENCES comments(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (reply_to_user_id) REFERENCES users(id) ON DELETE SET NULL,
    INDEX idx_sub_comments_comment (comment_id, created_at ASC),
    INDEX idx_sub_comments_user (user_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- POST VOTES
-- ============================================================
CREATE TABLE IF NOT EXISTS post_votes (
    id CHAR(36) NOT NULL PRIMARY KEY,
    post_id CHAR(36) NOT NULL,
    user_id CHAR(36) NOT NULL,
    vote_type ENUM('up', 'down') NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE KEY uq_post_vote (post_id, user_id),
    INDEX idx_post_votes_post (post_id),
    INDEX idx_post_votes_user (user_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- COMMENT VOTES
-- ============================================================
CREATE TABLE IF NOT EXISTS comment_votes (
    id CHAR(36) NOT NULL PRIMARY KEY,
    comment_id CHAR(36) NOT NULL,
    user_id CHAR(36) NOT NULL,
    vote_type ENUM('up', 'down') NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (comment_id) REFERENCES comments(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE KEY uq_comment_vote (comment_id, user_id),
    INDEX idx_comment_votes_comment (comment_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- SUB COMMENT VOTES
-- ============================================================
CREATE TABLE IF NOT EXISTS sub_comment_votes (
    id CHAR(36) NOT NULL PRIMARY KEY,
    sub_comment_id CHAR(36) NOT NULL,
    user_id CHAR(36) NOT NULL,
    vote_type ENUM('up', 'down') NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (sub_comment_id) REFERENCES sub_comments(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE KEY uq_sub_comment_vote (sub_comment_id, user_id),
    INDEX idx_sub_comment_votes_sc (sub_comment_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- POST STATUS HISTORY
-- ============================================================
CREATE TABLE IF NOT EXISTS post_status_history (
    id CHAR(36) NOT NULL PRIMARY KEY,
    post_id CHAR(36) NOT NULL,
    user_id CHAR(36) NOT NULL,
    old_status ENUM('pending', 'in_progress', 'closed') NOT NULL,
    new_status ENUM('pending', 'in_progress', 'closed') NOT NULL,
    note TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    INDEX idx_status_history_post (post_id, created_at DESC)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- CHATS (AI Conversations)
-- ============================================================
CREATE TABLE IF NOT EXISTS chats (
    id CHAR(36) NOT NULL PRIMARY KEY,
    user_id CHAR(36) NOT NULL,
    title VARCHAR(255) NOT NULL DEFAULT 'New Chat',
    chat_type ENUM('general', 'agentic') NOT NULL DEFAULT 'general',
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    message_count INT NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    INDEX idx_chats_user (user_id, updated_at DESC)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- CHAT MESSAGES (Encrypted)
-- ============================================================
CREATE TABLE IF NOT EXISTS chat_messages (
    id CHAR(36) NOT NULL PRIMARY KEY,
    chat_id CHAR(36) NOT NULL,
    user_id CHAR(36) NOT NULL,
    role ENUM('user', 'assistant', 'system') NOT NULL,
    content_enc TEXT NOT NULL,
    tool_calls_enc TEXT,
    tool_results_enc TEXT,
    has_tool_calls BOOLEAN NOT NULL DEFAULT FALSE,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (chat_id) REFERENCES chats(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    INDEX idx_messages_chat (chat_id, created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- USER AUTH LOGS
-- ============================================================
CREATE TABLE IF NOT EXISTS user_auth_logs (
    id CHAR(36) NOT NULL PRIMARY KEY,
    user_id CHAR(36) NOT NULL,
    action ENUM('login', 'logout', 'register', 'email_verify', 'verification_sent') NOT NULL,
    ip_address VARCHAR(45),
    user_agent TEXT,
    success BOOLEAN NOT NULL DEFAULT TRUE,
    failure_reason VARCHAR(255),
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    INDEX idx_auth_logs_user (user_id, created_at DESC)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- USER ACTIVITY LOGS
-- ============================================================
CREATE TABLE IF NOT EXISTS user_activity_logs (
    id CHAR(36) NOT NULL PRIMARY KEY,
    user_id CHAR(36) NOT NULL,
    action ENUM('create', 'read', 'update', 'delete') NOT NULL,
    feature VARCHAR(100) NOT NULL,
    entity_type VARCHAR(100) NOT NULL,
    entity_id CHAR(36),
    details TEXT,
    ip_address VARCHAR(45),
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    INDEX idx_activity_user (user_id, created_at DESC),
    INDEX idx_activity_entity (entity_type, entity_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
