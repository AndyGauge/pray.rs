use sqlx::SqlitePool;
use time::OffsetDateTime;
use uuid::Uuid;

use thanksgivings_core::{OAuthProvider, User, UserId};

struct UserRow {
    id: String, display_name: String, email: String,
    avatar_url: Option<String>, provider: String,
    provider_id: String, created_at: String,
}

impl From<UserRow> for User {
    fn from(r: UserRow) -> Self {
        User {
            id: Uuid::parse_str(&r.id).unwrap(),
            display_name: r.display_name,
            email: r.email,
            avatar_url: r.avatar_url,
            provider: if r.provider == "google" { OAuthProvider::Google } else { OAuthProvider::Facebook },
            provider_id: r.provider_id,
            created_at: OffsetDateTime::parse(&r.created_at, &time::format_description::well_known::Rfc3339)
                .unwrap_or(OffsetDateTime::UNIX_EPOCH),
        }
    }
}

pub async fn upsert_oauth_user(
    pool: &SqlitePool,
    provider: &OAuthProvider,
    provider_id: &str,
    email: &str,
    display_name: &str,
    avatar_url: Option<&str>,
) -> Result<User, sqlx::Error> {
    let id   = Uuid::new_v4().to_string();
    let prov = provider.to_string();
    let now  = OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339).unwrap();

    sqlx::query!(
        "INSERT INTO users (id, display_name, email, avatar_url, provider, provider_id, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(provider, provider_id) DO UPDATE SET
             display_name = excluded.display_name,
             avatar_url   = excluded.avatar_url",
        id, display_name, email, avatar_url, prov, provider_id, now,
    )
    .execute(pool)
    .await?;

    fetch_by_provider(pool, provider, provider_id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)
}

pub async fn fetch_by_id(pool: &SqlitePool, id: UserId) -> Result<Option<User>, sqlx::Error> {
    let id = id.to_string();
    Ok(
        sqlx::query_as!(
            UserRow,
            r#"SELECT id AS "id!", display_name, email, avatar_url, provider, provider_id, created_at
               FROM users WHERE id = ?"#,
            id,
        )
            .fetch_optional(pool)
            .await?
            .map(User::from),
    )
}

pub async fn delete_account(pool: &SqlitePool, id: UserId) -> Result<(), sqlx::Error> {
    let id_str = id.to_string();
    sqlx::query!("DELETE FROM posts WHERE author_id = ?", id_str)
        .execute(pool)
        .await?;
    sqlx::query!("DELETE FROM users WHERE id = ?", id_str)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn fetch_by_provider(
    pool: &SqlitePool,
    provider: &OAuthProvider,
    provider_id: &str,
) -> Result<Option<User>, sqlx::Error> {
    let provider = provider.to_string();
    Ok(
        sqlx::query_as!(
            UserRow,
            r#"SELECT id AS "id!", display_name, email, avatar_url, provider, provider_id, created_at
               FROM users WHERE provider = ? AND provider_id = ?"#,
            provider, provider_id,
        )
            .fetch_optional(pool)
            .await?
            .map(User::from),
    )
}
