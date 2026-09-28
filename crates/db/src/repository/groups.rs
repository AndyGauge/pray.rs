use sqlx::SqlitePool;
use time::OffsetDateTime;
use uuid::Uuid;

use thanksgivings_core::{Group, GroupId, UserId};

struct GroupRow { id: String, name: String, owner_id: String, created_at: String }

impl From<GroupRow> for Group {
    fn from(r: GroupRow) -> Self {
        Group {
            id: Uuid::parse_str(&r.id).unwrap(),
            name: r.name,
            owner_id: Uuid::parse_str(&r.owner_id).unwrap(),
            created_at: OffsetDateTime::parse(&r.created_at, &time::format_description::well_known::Rfc3339)
                .unwrap_or(OffsetDateTime::UNIX_EPOCH),
        }
    }
}

pub async fn list_for_user(pool: &SqlitePool, user_id: UserId) -> Result<Vec<Group>, sqlx::Error> {
    let uid = user_id.to_string();
    Ok(
        sqlx::query_as!(
            GroupRow,
            r#"SELECT g.id AS "id!", g.name, g.owner_id, g.created_at
               FROM groups g JOIN memberships m ON m.group_id = g.id
               WHERE m.user_id = ? ORDER BY g.name"#,
            uid,
        )
        .fetch_all(pool)
        .await?
        .into_iter().map(Group::from).collect(),
    )
}

pub async fn create(pool: &SqlitePool, name: &str, owner_id: UserId) -> Result<Group, sqlx::Error> {
    let id  = Uuid::new_v4().to_string();
    let uid = owner_id.to_string();
    let now = OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339).unwrap();

    sqlx::query!("INSERT INTO groups (id, name, owner_id, created_at) VALUES (?, ?, ?, ?)", id, name, uid, now)
        .execute(pool).await?;

    sqlx::query!("INSERT INTO memberships (user_id, group_id, joined_at) VALUES (?, ?, ?)", uid, id, now)
        .execute(pool).await?;

    Ok(Group {
        id: Uuid::parse_str(&id).unwrap(),
        name: name.to_string(),
        owner_id,
        created_at: OffsetDateTime::parse(&now, &time::format_description::well_known::Rfc3339).unwrap(),
    })
}

pub async fn add_member(pool: &SqlitePool, group_id: GroupId, user_id: UserId) -> Result<(), sqlx::Error> {
    let now = OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339).unwrap();
    let (uid, gid) = (user_id.to_string(), group_id.to_string());
    sqlx::query!("INSERT OR IGNORE INTO memberships (user_id, group_id, joined_at) VALUES (?, ?, ?)", uid, gid, now)
        .execute(pool).await?;
    Ok(())
}

pub async fn is_member(pool: &SqlitePool, user_id: UserId, group_id: GroupId) -> Result<bool, sqlx::Error> {
    let (uid, gid) = (user_id.to_string(), group_id.to_string());
    let row = sqlx::query!(
        "SELECT COUNT(*) AS cnt FROM memberships WHERE user_id = ? AND group_id = ?",
        uid, gid,
    )
    .fetch_one(pool).await?;
    Ok(row.cnt > 0)
}
