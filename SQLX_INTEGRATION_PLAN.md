# SQLx Integration Plan

## Overview

This plan adds the simplest SQLx setup to the server using PostgreSQL. The
goal is to persist guest profile data with minimal friction and a clear
migration path to a real auth system later.

## Assumptions

- PostgreSQL is available (managed or self-hosted).
- The server owns migrations and runs them on startup.
- The database is local to the server process.

## Environment Setup

Use a managed or self-hosted PostgreSQL connection string:

```bash
DATABASE_URL="postgres://USER:PASSWORD@HOST:PORT/jet_raiders"
```

## Dependencies

Add SQLx with PostgreSQL + Tokio support:

```toml
[dependencies]
sqlx = { version = "0.7", features = ["runtime-tokio", "postgres", "macros", "migrate"] }
dotenvy = "0.15"
```

## Migrations

Create a minimal table for guest profiles:

```sql
CREATE TABLE IF NOT EXISTS guest_profiles (
    guest_id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    metadata TEXT NOT NULL DEFAULT '{}'
);
```

## Database Module

Add a DB module that creates a small PostgreSQL pool:

```rust
use sqlx::{postgres::PgPoolOptions, PgPool};

// Build a connection pool for PostgreSQL.
pub async fn connect_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    // Keep pool small for local development.
    PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
}
```

## AppState Wiring

Store the pool in `AppState`:

```rust
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    // Existing fields...
    pub db: PgPool, // Shared DB pool for simple persistence.
}
```

Run migrations on startup:

```rust
use dotenvy::dotenv;
use sqlx::migrate::Migrator;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

#[tokio::main]
async fn main() {
    dotenv().ok();

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    let db = db::connect_pool(&database_url)
        .await
        .expect("failed to connect to database");

    MIGRATOR.run(&db).await.expect("failed to run migrations");

    let state = Arc::new(AppState {
        db,
        // Existing fields...
    });

    // Existing server startup...
}
```

## Guest Profile Upsert

Add a simple upsert helper:

```rust
use sqlx::PgPool;

// Save or update a guest profile.
pub async fn upsert_guest(
    db: &PgPool,
    guest_id: &str,
    display_name: &str,
    metadata_json: &str,
) -> Result<(), sqlx::Error> {
    // Store metadata as JSON text for flexibility.
    sqlx::query!(
        r#"
        INSERT INTO guest_profiles (guest_id, display_name, metadata)
        VALUES (?1, ?2, ?3)
        ON CONFLICT(guest_id) DO UPDATE SET
            display_name = excluded.display_name,
            metadata = excluded.metadata
        "#,
        guest_id,
        display_name,
        metadata_json
    )
    .execute(db)
    .await?;

    Ok(())
}
```

## Next Steps

1. Decide between SQLite or Postgres for the first integration.
2. Implement the DB module and migrations.
3. Wire up guest profile inserts on `Join`.
