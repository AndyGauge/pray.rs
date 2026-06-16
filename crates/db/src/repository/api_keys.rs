use sqlx::SqlitePool;
use uuid::Uuid;
use thanksgivings_core::UserId;

#[derive(Debug, Clone)]
pub struct ApiKey {
    pub id:      String,
    pub user_id: UserId,
    pub name:    String,
}

pub fn hash_key(raw: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(raw.as_bytes());
    format!("{digest:x}")
}

/// Create a new API key. Returns (id, raw_key) — raw key is shown once only.
pub async fn create(pool: &SqlitePool, user_id: UserId, name: &str)
    -> Result<(String, String), sqlx::Error>
{
    let id      = Uuid::new_v4().to_string();
    let raw     = format!("prs_{}", Uuid::new_v4().simple());
    let hash    = hash_key(&raw);
    let uid_str = user_id.to_string();

    sqlx::query(
        "INSERT INTO api_keys (id, user_id, name, key_hash) VALUES (?, ?, ?, ?)",
    )
    .bind(&id).bind(&uid_str).bind(name).bind(&hash)
    .execute(pool).await?;

    Ok((id, raw))
}

/// Look up user_id for a raw Bearer key, updating last_used_at.
pub async fn authenticate(pool: &SqlitePool, raw: &str)
    -> Result<Option<UserId>, sqlx::Error>
{
    let hash = hash_key(raw);

    let row = sqlx::query(
        "SELECT id, user_id FROM api_keys WHERE key_hash = ?",
    )
    .bind(&hash)
    .fetch_optional(pool).await?;

    let Some(row) = row else { return Ok(None) };

    let id:      String = sqlx::Row::try_get(&row, "id")?;
    let user_id: String = sqlx::Row::try_get(&row, "user_id")?;

    sqlx::query(
        "UPDATE api_keys SET last_used_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id = ?",
    )
    .bind(&id)
    .execute(pool).await?;

    Ok(Uuid::parse_str(&user_id).ok())
}

/// List all API keys for a user (metadata only — never the raw key).
pub async fn list(pool: &SqlitePool, user_id: UserId)
    -> Result<Vec<ApiKey>, sqlx::Error>
{
    let uid_str = user_id.to_string();
    let rows = sqlx::query(
        "SELECT id, user_id, name FROM api_keys WHERE user_id = ? ORDER BY created_at DESC",
    )
    .bind(&uid_str)
    .fetch_all(pool).await?;

    Ok(rows.into_iter().filter_map(|row| {
        Some(ApiKey {
            id:      sqlx::Row::try_get(&row, "id").ok()?,
            user_id: Uuid::parse_str(&sqlx::Row::try_get::<String, _>(&row, "user_id").ok()?).ok()?,
            name:    sqlx::Row::try_get(&row, "name").ok()?,
        })
    }).collect())
}

/// Revoke a key — only if it belongs to the requesting user.
pub async fn revoke(pool: &SqlitePool, id: &str, user_id: UserId)
    -> Result<bool, sqlx::Error>
{
    let uid_str = user_id.to_string();
    let result = sqlx::query(
        "DELETE FROM api_keys WHERE id = ? AND user_id = ?",
    )
    .bind(id).bind(&uid_str)
    .execute(pool).await?;

    Ok(result.rows_affected() > 0)
}
