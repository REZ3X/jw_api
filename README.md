# jw_api

REST API backend for **JogjaWaskita (JW)** — a civic engagement and government department reporting platform.

Built with Rust and Axum.

## Stack

| Layer | Tech |
|-------|------|
| Framework | Axum 0.7 |
| Database | MariaDB (SQLx) |
| Auth | Google OAuth 2.0 + JWT |
| Email | Brevo SMTP (Lettre) |
| AI | Gemini API (flash-lite) |
| Encryption | AES-256-GCM + HKDF |
| Storage | Local filesystem |

## Project Structure

```
src/
├── main.rs              # Entry point, CORS, graceful shutdown
├── config.rs            # Environment config loader
├── crypto.rs            # AES-256-GCM encryption service
├── db.rs                # Connection pool + migration runner
├── error.rs             # Unified error type
├── state.rs             # Shared application state
├── middleware/
│   ├── api_key.rs       # External mode API key gate
│   ├── auth.rs          # JWT extractors (AuthUser → VerifiedUser → GovUser → DevUser)
│   └── activity_log.rs  # Auth + activity audit logging
├── models/              # Database rows, API request/response DTOs
├── routes/              # HTTP handlers grouped by domain
└── services/            # Business logic layer
```

## Setup

### Prerequisites

- Rust 1.75+
- MariaDB 10.6+

### Database

```sql
CREATE DATABASE jw_db CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
```

Migrations run automatically on startup.

### Environment

Copy `.env.example` to `.env` and fill in your credentials:

```bash
cp .env.example .env
```

Key variables:

| Variable | Description |
|----------|-------------|
| `DATABASE_URL` | MariaDB connection string |
| `JWT_SECRET` | Secret for signing JWT tokens |
| `GOOGLE_CLIENT_ID` / `GOOGLE_CLIENT_SECRET` | Google OAuth credentials |
| `GOOGLE_REDIRECT_URI` | OAuth callback URL |
| `GEMINI_API_KEY` | Comma-separated API keys with failover (e.g. `"key1, key2"`) |
| `GEMINI_MODEL` | Comma-separated model list with failover (e.g. `"gemini-3.1-flash-lite-preview, gemini-2.5-flash"`) |
| `ENCRYPTION_MASTER_KEY` | 64-char hex string (32 bytes) for AES-256 |
| `BREVO_SMTP_*` | Brevo SMTP credentials |
| `APP_MODE` | `internal` (dev) or `external` (prod) |
| `APP_PORT` | Server port (default: 8000) |
| `FRONTEND_URL` | Frontend origin for CORS + email links |
| `MAX_IMAGE_SIZE_MB` | Maximum image upload size limit (e.g. 10) |
| `MAX_VIDEO_SIZE_MB` | Maximum video upload size limit (e.g. 50) |

Generate an encryption key:

```bash
openssl rand -hex 32
```

### Run

```bash
cargo run
```

Server starts at `http://localhost:8000`. Verify with:

```bash
curl http://localhost:8000/health
```

## Testing

The project includes a performance-oriented testing framework built with `axum-test` and `tokio-test`. Tests enforce hard timing budgets (SLAs) and produce parseable audit output for regression tracking.

### Running Tests

```bash
# Unit tests (no external services needed)
cargo test

# Full suite including DB and API integration tests
cargo test -- --include-ignored

# Run a specific test file
cargo test --test test_perf_crypto
cargo test --test test_perf_routes -- --ignored

# Run a single test by name
cargo test perf_encrypt_decrypt_throughput
cargo test vote_toggle_and_switch -- --ignored
```

> **Note**: Integration tests load variables from `.env` automatically. To run live email tests, set `TEST_EMAIL_RECIPIENT=` in `.env`.

### Test Suite

| File | Requires | Coverage |
|------|----------|----------|
| `test_perf_crypto` | — | Encrypt/decrypt throughput (500x), concurrency (50 tasks), 1MB payload, malformed input, salt uniqueness |
| `test_perf_media` | — | Write throughput (20 files), boundary sizes, type detection, concurrent writes, delete verification |
| `test_perf_auth` | `.env` | JWT generation (100x), claims validation, token uniqueness (500x), role detection, avatar fallback |
| `test_errors_models` | — | All AppError → HTTP status mappings, response body shape, Display trait |
| `test_config_crypto` | — | Config parsing, crypto init, encrypt/decrypt roundtrip, wrong salt rejection |
| `test_perf_routes` | MariaDB | Health endpoint latency (p95), throughput (50 req), auth guard on 10 protected routes, 404 handling |
| `test_perf_database` | MariaDB | Connection pool latency (p95), migration idempotency, concurrent queries (20x), write/read consistency |
| `test_services` | MariaDB | Post create/delete lifecycle, comment count tracking, vote toggle/switch, bulk creation (10x), file upload limits |
| `test_perf_gemini` | Gemini API | Response time budget, classification consistency (4/5), title invariants, concurrent calls, edge cases |
| `test_gemini` | Gemini API | Live generation, department classification, title generation, agent tool calling format |
| `test_routes` | MariaDB | Health endpoint structure, unauthorized route rejection |
| `test_email` | Brevo SMTP | Live verification email send |
| `test_auth_chat` | MariaDB | JWT-protected chat endpoint rejection |
| `test_database` | MariaDB | Connection pool, migration runner |

## API Reference

### Auth

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/auth/google/url` | — | Get Google OAuth redirect URL |
| GET | `/api/auth/google/callback?code=` | — | Exchange OAuth code for JWT |
| GET | `/api/auth/verify-email?token=` | — | Verify email address |
| POST | `/api/auth/resend-verification` | JWT | Resend verification email |
| GET | `/api/auth/me` | JWT | Get current user |
| PUT | `/api/auth/me` | Verified | Update profile (name, bio, birth) |

### Users

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/users/search?q=` | — | Search users by name or username |
| GET | `/api/users/:username` | — | Public profile |
| GET | `/api/users/:username/posts` | — | User's public posts |
| POST | `/api/users/me/avatar` | Verified | Upload custom avatar (multipart) |
| DELETE | `/api/users/me/avatar` | Verified | Revert to Google avatar |

### Posts

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/api/posts` | Verified | Create post (multipart: caption + media) |
| GET | `/api/posts` | Optional | List posts (filters: department, status, tag, search, sort; defaults to random within last 48 hours if unfiltered) |
| GET | `/api/posts/me` | Verified | List own posts (includes private) |
| GET | `/api/posts/:id` | Optional | Get single post |
| PUT | `/api/posts/:id` | Verified | Update post (caption editable within 24h) |
| DELETE | `/api/posts/:id` | Verified | Delete own post |
| POST | `/api/posts/:id/classify` | Verified | AI-classify department for a caption |

### Comments

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/api/comments/post/:post_id` | Verified | Create comment |
| GET | `/api/comments/post/:post_id` | — | List comments (sort: recent, most_upvote, popular) |
| PUT | `/api/comments/:id` | Verified | Edit own comment |
| DELETE | `/api/comments/:id` | Verified | Delete own comment |
| POST | `/api/comments/:id/pin` | Verified | Toggle pin (post owner only) |
| POST | `/api/comments/:id/replies` | Verified | Create reply |
| GET | `/api/comments/:id/replies` | — | List replies |

### Votes

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/api/votes/post/:id` | Verified | Toggle vote on post |
| POST | `/api/votes/comment/:id` | Verified | Toggle vote on comment |
| POST | `/api/votes/subcomment/:id` | Verified | Toggle vote on reply |

Request body: `{ "vote_type": "up" | "down" }`

### Department (Gov Users)

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/departments/dashboard` | Gov | Department stats |
| GET | `/api/departments/posts` | Gov | Posts routed to your department |
| PUT | `/api/departments/posts/:id/status` | Gov | Update post status (pending → in_progress → closed) |
| POST | `/api/departments/posts/:id/respond` | Gov | Official response with optional image (multipart) |
| GET | `/api/departments/all-posts` | city_major_gov | Cross-department view |

### Chat (AI)

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/api/chats` | Verified | Create chat (`general` or `agentic`) |
| GET | `/api/chats` | Verified | List chats |
| GET | `/api/chats/:id` | Verified | Get chat |
| PUT | `/api/chats/:id` | Verified | Update chat (title, active) |
| DELETE | `/api/chats/:id` | Verified | Delete chat |
| GET | `/api/chats/:id/messages` | Verified | Get decrypted messages |
| POST | `/api/chats/:id/messages` | Verified | Send message (AI responds) |

Chat types:
- **general** — Conversational civic engagement assistant
- **agentic** — Role-aware tool-calling mode with access to platform data

All chat messages are encrypted at rest with per-user derived keys (AES-256-GCM + HKDF).

#### Agentic Tools

Tools available depend on the user's role:

**Shared (all roles)**

| Tool | Params | Description |
|------|--------|-------------|
| `GET_TRENDING_TAGS` | — | Trending hashtags |
| `GET_PLATFORM_STATS` | — | Platform-wide statistics |
| `GET_DEPARTMENT_STATS` | — | Per-department breakdown |
| `SEARCH_POSTS` | department?, status?, tag?, search?, limit? | Search public posts |

**Citizen (basic / dev)**

| Tool | Params | Description |
|------|--------|-------------|
| `GET_MY_POSTS` | status?, limit? | Own submitted reports |
| `GET_MY_UNRESPONDED_POSTS` | department? | Reports not yet resolved |
| `CREATE_POST_DRAFT` | caption, location, department?, is_private? | Draft a report via chat (validates required fields, asks user for missing ones) |

**Government roles**

| Tool | Params | Description |
|------|--------|-------------|
| `GET_DEPT_QUEUE` | status?, limit? | Reports awaiting action in your department |
| `GET_DEPT_TODAY_STATS` | — | Today's report counts + backlog |
| `GET_DEPT_RESPONSE_HISTORY` | limit? | Your recent status changes |
| `SEARCH_DEPT_POSTS` | status?, search?, tag?, days?, limit? | Search within your department |
| `GET_DEPT_TRENDS` | days? | Daily report volume + top tags for your department |

Gov accounts cannot use citizen tools (`GET_MY_POSTS`, `CREATE_POST_DRAFT`).

### Analytics

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/analytics/trending-tags` | — | Trending hashtags |
| GET | `/api/analytics/stats` | — | Platform-wide statistics |
| GET | `/api/analytics/department-stats` | — | Per-department breakdown |
| GET | `/api/analytics/trends?days=30` | — | Post volume over time |

### Dev Admin

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/dev/users` | Dev | List users (filter by role, search) |
| PUT | `/api/dev/users/:id/role` | Dev | Assign user role |
| GET | `/api/dev/analytics/overview` | Dev | Full analytics overview |

### Logs (Dev Only)

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/logs/auth` | Dev | Auth event logs |
| GET | `/api/logs/activity` | Dev | Activity audit logs |

### Static Files

Uploaded media is served from `/uploads/*`.

## Roles

| Role | Access |
|------|--------|
| `basic` | Create posts, comment, vote, AI chat |
| `city_major_gov` | Department dashboard + cross-department view |
| `fire_department` | Department dashboard for fire reports |
| `health_department` | Department dashboard for health reports |
| `environment_department` | Department dashboard for environment reports |
| `police_department` | Department dashboard for police reports |
| `dev` | Full admin: user management, logs, analytics |

## Auth Flow

```
1. Client  → GET /api/auth/google/url
2. Client  → Redirect to Google consent screen
3. Google  → Redirect back with ?code=
4. Client  → GET /api/auth/google/callback?code=XXX
5. Server  → Returns JWT + user + is_new_user
6. Client  → Include JWT in Authorization: Bearer <token>
```

New users receive a verification email. Some endpoints require `VerifiedUser`.

## License

Private. All rights reserved.
