use sqlx::SqlitePool;
use time::OffsetDateTime;
use uuid::Uuid;

use thanksgivings_core::{GroupId, UserId};

pub async fn get_for_group(pool: &SqlitePool, group_id: GroupId) -> Result<Option<String>, sqlx::Error> {
    let gid = group_id.to_string();
    let row = sqlx::query!("SELECT token FROM group_join_tokens WHERE group_id = ?", gid)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| r.token))
}

pub async fn create_or_replace(pool: &SqlitePool, group_id: GroupId, user_id: UserId) -> Result<String, sqlx::Error> {
    let token = format!("grp_{}", Uuid::new_v4().simple());
    let id    = Uuid::new_v4().to_string();
    let now   = OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap();

    let (gid, uid) = (group_id.to_string(), user_id.to_string());
    sqlx::query!("DELETE FROM group_join_tokens WHERE group_id = ?", gid)
        .execute(pool).await?;

    sqlx::query!(
        "INSERT INTO group_join_tokens (id, group_id, created_by, token, created_at) VALUES (?, ?, ?, ?, ?)",
        id, gid, uid, token, now,
    )
    .execute(pool).await?;

    Ok(token)
}

pub async fn find_group(pool: &SqlitePool, token: &str) -> Result<Option<GroupId>, sqlx::Error> {
    let row = sqlx::query!("SELECT group_id FROM group_join_tokens WHERE token = ?", token)
        .fetch_optional(pool)
        .await?;
    Ok(row.and_then(|r| Uuid::parse_str(&r.group_id).ok()))
}
