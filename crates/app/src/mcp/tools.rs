use mcp_authorization::Proof;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use thanksgivings_core::{ContactType, NewPost, PostKind, UserId, Visibility, VisibilityFilter};
use uuid::Uuid;

use super::{capabilities::*, pagination::Pagination};

fn ok(text: impl Into<String>) -> Value {
    json!({ "content": [{ "type": "text", "text": text.into() }] })
}
fn err(msg: impl Into<String>) -> Value {
    json!({ "content": [{ "type": "text", "text": msg.into() }], "isError": true })
}

fn vis_label(v: &Visibility) -> String {
    match v {
        Visibility::Private   => "private".to_string(),
        Visibility::Public    => "public".to_string(),
        Visibility::Group(id) => format!("group:{id}"),
    }
}

// ── ReadPublic ────────────────────────────────────────────────────────────────

pub async fn list_public_prayers(
    _proof: Proof<ReadPublic>,
    pool: &SqlitePool,
    page: Pagination,
) -> Value {
    match thanksgivings_db::repository::posts::list_public(pool).await {
        Ok(posts) => ok(page.render(&posts, "\n\n---\n\n", |p| {
            format!("[{}] {}\n{}", p.id, p.created_at, p.content)
        })),
        Err(e) => err(e.to_string()),
    }
}

// ── ReadOwn ───────────────────────────────────────────────────────────────────

pub async fn list_my_prayers(
    _proof: Proof<ReadOwn>,
    user_id: UserId,
    pool: &SqlitePool,
    page: Pagination,
) -> Value {
    match thanksgivings_db::repository::posts::list_all_for_author(pool, user_id).await {
        Ok(posts) => ok(page.render(&posts, "\n\n---\n\n", |p| {
            format!("[{}] ({}) {}\n{}", p.id, vis_label(&p.visibility), p.created_at, p.content)
        })),
        Err(e) => err(e.to_string()),
    }
}

// ── WriteOwn ──────────────────────────────────────────────────────────────────

pub async fn create_prayer(
    _proof: Proof<WriteOwn>,
    user_id: UserId,
    pool: &SqlitePool,
    content: String,
    visibility: String,
) -> Value {
    let vis = match visibility.as_str() {
        "public" => Visibility::Public,
        _        => Visibility::Private,
    };
    let new = NewPost { kind: PostKind::Prayer, content, visibility: vis, prayer_id: None };
    match thanksgivings_db::repository::posts::create(pool, user_id, new).await {
        Ok(post) => ok(format!("Prayer created: {}", post.id)),
        Err(e)   => err(e.to_string()),
    }
}

pub async fn set_visibility(
    _proof: Proof<WriteOwn>,
    user_id: UserId,
    pool: &SqlitePool,
    post_id: String,
    visibility: String,
) -> Value {
    // Validate visibility string
    let vis = match visibility.as_str() {
        "public"  => "public".to_string(),
        "private" => "private".to_string(),
        other => match Uuid::parse_str(other) {
            Ok(_)  => other.to_string(), // valid group UUID
            Err(_) => return err("visibility must be 'public', 'private', or a group UUID"),
        },
    };
    match thanksgivings_db::repository::posts::set_visibility(pool, &post_id, user_id, &vis).await {
        Ok(true)  => ok(format!("Visibility set to '{vis}'.")),
        Ok(false) => err("Prayer not found or not yours."),
        Err(e)    => err(e.to_string()),
    }
}

pub async fn delete_prayer(
    _proof: Proof<WriteOwn>,
    user_id: UserId,
    pool: &SqlitePool,
    post_id: String,
) -> Value {
    match thanksgivings_db::repository::posts::delete(pool, &post_id, user_id).await {
        Ok(true)  => ok("Prayer deleted."),
        Ok(false) => err("Prayer not found or not yours."),
        Err(e)    => err(e.to_string()),
    }
}

pub async fn edit_prayer(
    _proof: Proof<WriteOwn>,
    user_id: UserId,
    pool: &SqlitePool,
    post_id: String,
    content: String,
) -> Value {
    if content.trim().is_empty() {
        return err("content cannot be empty");
    }
    match thanksgivings_db::repository::posts::update_content(pool, &post_id, user_id, &content).await {
        Ok(true)  => ok("Prayer updated."),
        Ok(false) => err("Prayer not found or not yours."),
        Err(e)    => err(e.to_string()),
    }
}

// ── ReadGroup ─────────────────────────────────────────────────────────────────

pub async fn list_group_prayers(
    _proof: Proof<ReadGroup>,
    user_id: UserId,
    pool: &SqlitePool,
    group_id: String,
    page: Pagination,
) -> Value {
    let gid = match Uuid::parse_str(&group_id) {
        Ok(id) => id,
        Err(_) => return err("Invalid group ID."),
    };
    let groups = match thanksgivings_db::repository::groups::list_for_user(pool, user_id).await {
        Ok(g) => g, Err(e) => return err(e.to_string()),
    };
    if !groups.iter().any(|g| g.id == gid) {
        return err("You are not a member of that group.");
    }
    match thanksgivings_db::repository::posts::list_for_viewer(
        pool, user_id, &VisibilityFilter::Group(gid),
    ).await {
        Ok(posts) => ok(page.render(&posts, "\n\n---\n\n", |p| {
            format!("[{}] {}\n{}", p.id, p.created_at, p.content)
        })),
        Err(e) => err(e.to_string()),
    }
}

// ── ManageGroups ──────────────────────────────────────────────────────────────

pub async fn list_my_groups(_proof: Proof<ManageGroups>, user_id: UserId, pool: &SqlitePool) -> Value {
    match thanksgivings_db::repository::groups::list_for_user(pool, user_id).await {
        Ok(gs) if gs.is_empty() => ok("You are not in any groups yet."),
        Ok(gs) => ok(gs.iter().map(|g| format!("[{}] {}", g.id, g.name)).collect::<Vec<_>>().join("\n")),
        Err(e) => err(e.to_string()),
    }
}

pub async fn create_group(
    _proof: Proof<ManageGroups>,
    user_id: UserId,
    pool: &SqlitePool,
    name: String,
) -> Value {
    if name.trim().is_empty() { return err("name cannot be empty"); }
    match thanksgivings_db::repository::groups::create(pool, &name, user_id).await {
        Ok(g)  => ok(format!("Group created: [{}] {}", g.id, g.name)),
        Err(e) => err(e.to_string()),
    }
}

pub async fn invite_to_group(
    _proof: Proof<ManageGroups>,
    user_id: UserId,
    pool: &SqlitePool,
    group_id: String,
    email: String,
) -> Value {
    let gid = match Uuid::parse_str(&group_id) {
        Ok(id) => id,
        Err(_) => return err("Invalid group ID."),
    };

    // Caller must be a member
    let groups = match thanksgivings_db::repository::groups::list_for_user(pool, user_id).await {
        Ok(g) => g, Err(e) => return err(e.to_string()),
    };
    let group = match groups.iter().find(|g| g.id == gid) {
        Some(g) => g.clone(),
        None    => return err("You are not a member of that group."),
    };

    // Create the invitation record
    let inv = match thanksgivings_db::repository::invitations::create(
        pool, gid, user_id, &email, &ContactType::Email,
    ).await {
        Ok(i)  => i,
        Err(e) => return err(e.to_string()),
    };

    // Fetch inviter name for the email
    let inviter_name = match thanksgivings_db::repository::users::fetch_by_id(pool, user_id).await {
        Ok(Some(u)) => u.display_name,
        _           => "Someone".to_string(),
    };

    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "https://pray.rs".to_string());
    let invite_url = format!("{}/invite/{}", base_url, inv.token);

    match crate::email::mailer::send_group_invite(&email, &inviter_name, &group.name, &invite_url).await {
        Ok(())  => ok(format!("Invitation sent to {}.", email)),
        Err(e)  => err(format!("Invite recorded but email failed: {e}")),
    }
}

// ── ShareToGroup ──────────────────────────────────────────────────────────────

pub async fn share_to_group(
    _proof: Proof<ShareToGroup>,
    user_id: UserId,
    pool: &SqlitePool,
    post_id: String,
    group_id: String,
) -> Value {
    let gid = match Uuid::parse_str(&group_id) {
        Ok(id) => id,
        Err(_) => return err("Invalid group ID."),
    };
    let groups = match thanksgivings_db::repository::groups::list_for_user(pool, user_id).await {
        Ok(g) => g, Err(e) => return err(e.to_string()),
    };
    if !groups.iter().any(|g| g.id == gid) {
        return err("You are not a member of that group.");
    }
    match thanksgivings_db::repository::posts::set_visibility(
        pool, &post_id, user_id, &group_id,
    ).await {
        Ok(true)  => ok("Prayer shared to group."),
        Ok(false) => err("Prayer not found or not yours."),
        Err(e)    => err(e.to_string()),
    }
}
