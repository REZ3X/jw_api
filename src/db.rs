use sqlx::mysql::{MySqlPool, MySqlPoolOptions};

use crate::config::DatabaseConfig;

pub async fn create_pool(config: &DatabaseConfig) -> Result<MySqlPool, sqlx::Error> {
    let pool = MySqlPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(2)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .idle_timeout(std::time::Duration::from_secs(600))
        .max_lifetime(std::time::Duration::from_secs(1800))
        .connect(&config.url)
        .await?;

    tracing::info!("Database connection pool established");
    Ok(pool)
}

pub async fn run_migrations(pool: &MySqlPool) -> Result<(), anyhow::Error> {
    let migrations: &[&str] = &[
        include_str!("../migrations/001_initial_schema.sql"),
    ];

    for migration_sql in migrations {
        run_migration_sql(pool, migration_sql).await?;
    }

    tracing::info!("Database migrations completed");
    Ok(())
}

async fn run_migration_sql(pool: &MySqlPool, sql: &str) -> Result<(), anyhow::Error> {
    for statement in sql.split(';') {
        let cleaned: String = statement
            .lines()
            .filter(|line| {
                let t = line.trim();
                !t.is_empty() && !t.starts_with("--")
            })
            .collect::<Vec<_>>()
            .join("\n");
        let cleaned = cleaned.trim();

        if cleaned.is_empty() || cleaned.starts_with("SET ") {
            continue;
        }

        sqlx::query(cleaned)
            .execute(pool)
            .await
            .map_err(|e| {
                tracing::warn!("Migration statement failed: {}", e);
                e
            })
            .ok();
    }
    Ok(())
}
