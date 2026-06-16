use sqlx::SqlitePool;
use time::OffsetDateTime;
use uuid::Uuid;

use thanksgivings_core::{
    ContactType, GroupId, Invitation, InvitationStatus, InviteSummary, UserId,
};

#[derive(sqlx::FromRow)]
struct InvitationRow {
    id: String, group_id: String, invited_by: String,
    contact: String, contact_type: String, token: String,
    status: String, created_at: String, expires_at: String,
}

fn parse_contact_type(s: &str) -> ContactType {
    if s == "phone" { ContactType::Phone } else { ContactType::Email }
}
fn parse_status(s: &str) -> InvitationStatus {
    match s {
        "accepted" => InvitationStatus::Accepted,
        "declined" => InvitationStatus::Declined,
        _          => InvitationStatus::Pending,
    }
}
fn parse_time(s: &str) -> OffsetDateTime {
    OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339)
        .unwrap_or(OffsetDateTime::UNIX_EPOCH)
}

impl From<InvitationRow> for Invitation {
    fn from(r: InvitationRow) -> Self {
        Invitation {
            id:           Uuid::parse_str(&r.id).unwrap(),
            group_id:     Uuid::parse_str(&r.group_id).unwrap(),
            invited_by:   Uuid::parse_str(&r.invited_by).unwrap(),
            contact:      r.contact,
            contact_type: parse_contact_type(&r.contact_type),
            token:        r.token,
            status:       parse_status(&r.status),
            created_at:   parse_time(&r.created_at),
            expires_at:   parse_time(&r.expires_at),
        }
    }
}

/// Generates a URL-safe random token (no external dep needed).
fn new_token() -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(32);
    let bytes = Uuid::new_v4().as_bytes().to_vec();
    let bytes2 = Uuid::new_v4().as_bytes().to_vec();
    for b in bytes.iter().chain(bytes2.iter()) {
        write!(s, "{:02x}", b).unwrap();
    }
    s
}

pub async fn create(
    pool: &SqlitePool,
    group_id: GroupId,
    invited_by: UserId,
    contact: &str,
    contact_type: &ContactType,
) -> Result<Invitation, sqlx::Error> {
    let id    = Uuid::new_v4().to_string();
    let token = new_token();
    let now   = OffsetDateTime::now_utc();
    let exp   = now + time::Duration::days(7);
    let fmt   = &time::format_description::well_known::Rfc3339;
    let now_s = now.format(fmt).unwrap();
    let exp_s = exp.format(fmt).unwrap();
    let ct    = if *contact_type == ContactType::Phone { "phone" } else { "email" };

    sqlx::query(
        "INSERT INTO invitations (id, group_id, invited_by, contact, contact_type, token, created_at, expires_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&id).bind(group_id.to_string()).bind(invited_by.to_string())
    .bind(contact).bind(ct).bind(&token).bind(&now_s).bind(&exp_s)
    .execute(pool).await?;

    Ok(Invitation {
        id:           Uuid::parse_str(&id).unwrap(),
        group_id,
        invited_by,
        contact:      contact.to_string(),
        contact_type: contact_type.clone(),
        token,
        status:       InvitationStatus::Pending,
        created_at:   now,
        expires_at:   exp,
    })
}

pub async fn find_by_token(
    pool: &SqlitePool,
    token: &str,
) -> Result<Option<Invitation>, sqlx::Error> {
    Ok(
        sqlx::query_as::<_, InvitationRow>(
            "SELECT id, group_id, invited_by, contact, contact_type, token,
                    status, created_at, expires_at
             FROM invitations WHERE token = ? AND status = 'pending'
             AND expires_at > strftime('%Y-%m-%dT%H:%M:%SZ', 'now')"
        )
        .bind(token)
        .fetch_optional(pool)
        .await?
        .map(Invitation::from)
    )
}

pub async fn accept(
    pool: &SqlitePool,
    token: &str,
    user_id: UserId,
) -> Result<Option<GroupId>, sqlx::Error> {
    let inv = match find_by_token(pool, token).await? {
        Some(i) => i,
        None    => return Ok(None),
    };

    sqlx::query("UPDATE invitations SET status = 'accepted' WHERE token = ?")
        .bind(token).execute(pool).await?;

    let now = OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339).unwrap();
    sqlx::query(
        "INSERT OR IGNORE INTO memberships (user_id, group_id, joined_at) VALUES (?, ?, ?)"
    )
    .bind(user_id.to_string()).bind(inv.group_id.to_string()).bind(&now)
    .execute(pool).await?;

    Ok(Some(inv.group_id))
}

pub async fn list_pending_for_group(
    pool: &SqlitePool,
    group_id: GroupId,
) -> Result<Vec<InviteSummary>, sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct Row {
        id: String, contact: String, contact_type: String,
        invited_by_name: String, group_name: String, token: String,
    }
    let rows = sqlx::query_as::<_, Row>(
        "SELECT i.id, i.contact, i.contact_type, u.display_name AS invited_by_name,
                g.name AS group_name, i.token
         FROM invitations i
         JOIN users  u ON u.id = i.invited_by
         JOIN groups g ON g.id = i.group_id
         WHERE i.group_id = ? AND i.status = 'pending'
         AND i.expires_at > strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
         ORDER BY i.created_at DESC"
    )
    .bind(group_id.to_string())
    .fetch_all(pool).await?;

    Ok(rows.into_iter().map(|r| InviteSummary {
        id:              Uuid::parse_str(&r.id).unwrap(),
        contact:         r.contact,
        contact_type:    parse_contact_type(&r.contact_type),
        invited_by_name: r.invited_by_name,
        group_name:      r.group_name,
        token:           r.token,
    }).collect())
}
