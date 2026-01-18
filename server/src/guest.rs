use sqlx::PgPool;

// Upsert a guest profile into the database for simple persistence.
pub async fn upsert_guest(
    db: &PgPool,
    guest_id: &str,
    display_name: &str,
    metadata_json: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO guest_profiles (guest_id, display_name, metadata)
        VALUES ($1, $2, $3)
        ON CONFLICT (guest_id) DO UPDATE SET
            display_name = EXCLUDED.display_name,
            metadata = EXCLUDED.metadata
        "#,
    )
    .bind(guest_id)
    .bind(display_name)
    .bind(metadata_json)
    .execute(db)
    .await?;

    Ok(())
}
