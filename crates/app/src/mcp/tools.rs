use mcp_authorization::Proof;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use thanksgivings_core::{
    ContactType, NewPost, PostAction, PostState, PrayerCount, UserId, Visibility, VisibilityFilter,
};
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

/// An entry's lifecycle log as trailing lines — `→ thanksgiving (<when>): <note>` —
/// then its prayer tally, if anyone has prayed.
fn history_text(p: &thanksgivings_core::Post) -> String {
    let rfc3339 = &time::format_description::well_known::Rfc3339;
    let mut out: String = p.history.iter().map(|t| {
        let when = t.at.format(rfc3339).unwrap_or_default();
        match &t.note {
            Some(n) => format!("\n→ {} ({when}): {n}", t.to.as_str()),
            None    => format!("\n→ {} ({when})", t.to.as_str()),
        }
    }).collect();
    if p.prayers.total > 0 {
        out.push_str(&format!("\n🙏 {}", tally_text(p.prayers)));
    }
    out
}

fn tally_text(c: PrayerCount) -> String {
    format!(
        "{} prayer{} from {} {}",
        c.total, if c.total == 1 { "" } else { "s" },
        c.people, if c.people == 1 { "person" } else { "people" },
    )
}

// ── ReadPublic ────────────────────────────────────────────────────────────────

pub async fn list_public_prayers(
    _proof: Proof<ReadPublic>,
    pool: &SqlitePool,
    page: Pagination,
) -> Value {
    match thanksgivings_db::repository::posts::list_public(pool).await {
        Ok(posts) => {
            let mut result = ok(page.render(&posts, "\n\n---\n\n", |p| {
                format!("[{}] {} {}\n{}{}", p.id, p.state, p.created_at, p.content, history_text(p))
            }));
            // The same page as JSON (MCP `structuredContent`), for programs such as
            // the website build. Deliberately no author ids: a public collection
            // shouldn't let readers link prayers to one person.
            let (items, next_offset) = page.slice(&posts);
            result["structuredContent"] = json!({
                "prayers": items.iter().map(public_prayer_json).collect::<Vec<_>>(),
                "total": posts.len(),
                "next_offset": next_offset,
            });
            result
        }
        Err(e) => err(e.to_string()),
    }
}

/// A public prayer for machine consumers: content, state, dates (RFC 3339),
/// lifecycle history with notes, and the prayer tally. No author id.
fn public_prayer_json(p: &thanksgivings_core::Post) -> Value {
    let rfc3339 = |t: &time::OffsetDateTime| {
        t.format(&time::format_description::well_known::Rfc3339).unwrap_or_default()
    };
    json!({
        "id": p.id,
        "state": p.state.as_str(),
        "content": p.content,
        "created_at": rfc3339(&p.created_at),
        "history": p.history.iter().map(|t| json!({
            "from": t.from.as_str(),
            "to": t.to.as_str(),
            "note": t.note,
            "at": rfc3339(&t.at),
        })).collect::<Vec<_>>(),
        "prayers": { "total": p.prayers.total, "people": p.prayers.people },
    })
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
            format!("[{}] {} ({}) {}\n{}{}", p.id, p.state, vis_label(&p.visibility), p.created_at, p.content, history_text(p))
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
    state: String,
) -> Value {
    let vis = match visibility.as_str() {
        "public" => Visibility::Public,
        _        => Visibility::Private,
    };
    let state = match PostState::parse(&state) {
        Some(s) if s.is_initial() => s,
        _ => return err("state must be 'prayer' or 'thanksgiving'"),
    };
    let new = NewPost { state, content, visibility: vis };
    match thanksgivings_db::repository::posts::create(pool, user_id, new).await {
        Ok(post) => ok(format!("{} created: {}", post.state, post.id)),
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

/// An MCP tool that moves an entry into some state.
pub struct TransitionTool {
    pub name:        &'static str,
    pub description: &'static str,
    /// Description of the tool's optional `note` argument, which is logged
    /// with the move and shown after the entry.
    pub note:        &'static str,
}

/// The MCP tool that moves an entry into `to`, or `None` if AI clients get no
/// tool for it. The `match` has no wildcard on purpose: a new `PostState`
/// won't compile until you decide how MCP exposes it.
pub fn transition_tool(to: PostState) -> Option<TransitionTool> {
    match to {
        // Only a starting state today; nothing moves back into it. If the
        // machine ever allows that, `every_transition_target_has_a_tool` fails.
        PostState::Prayer => None,
        PostState::Thanksgiving => Some(TransitionTool {
            name:        "give_thanks",
            description: "Mark one of your prayers as answered, turning it into a thanksgiving. \
                          The prayer's text is kept; the note is recorded after it.",
            note:        "Optional: how the prayer was answered.",
        }),
        PostState::Released => Some(TransitionTool {
            name:        "release_prayer",
            description: "Release one of your prayers or thanksgivings. Released entries are hidden from every listing.",
            note:        "Optional: why it is being released.",
        }),
    }
}

/// The MCP tool for a `PostAction`.
pub enum ActionTool {
    /// A lifecycle move: `post_id` + optional `note`. Needs `WriteOwn`.
    Move(TransitionTool),
    /// "Praying now" (+1): `post_id` only. Needs `Pray`.
    PrayingNow { name: &'static str, description: &'static str },
}

impl ActionTool {
    pub fn name(&self) -> &'static str {
        match self {
            ActionTool::Move(t)                => t.name,
            ActionTool::PrayingNow { name, .. } => name,
        }
    }
}

/// How MCP exposes `action`, or `None` if it doesn't. No wildcard on purpose:
/// a new `PostAction` won't compile until you decide here.
pub fn action_tool(action: PostAction) -> Option<ActionTool> {
    match action {
        PostAction::MoveTo(to) => transition_tool(to).map(ActionTool::Move),
        PostAction::PrayingNow => Some(ActionTool::PrayingNow {
            name:        "pray_for",
            description: "Tell someone you're praying for their prayer right now (+1). Works on any \
                          prayer you can see — yours, your groups', or public ones — and every call \
                          counts, so call it each time you pray.",
        }),
    }
}

/// The action an MCP tool performs, looked up by tool name.
pub fn action_for_tool(name: &str) -> Option<PostAction> {
    PostAction::all().find(|&a| action_tool(a).is_some_and(|t| t.name() == name))
}

/// Move a prayer to thanksgiving (answered) or either to released (hidden),
/// logging the optional note with the move.
pub async fn transition_prayer(
    _proof: Proof<WriteOwn>,
    user_id: UserId,
    pool: &SqlitePool,
    post_id: String,
    to: PostState,
    note: Option<String>,
) -> Value {
    match thanksgivings_db::repository::posts::transition(pool, &post_id, user_id, to, note.as_deref()).await {
        Ok(true)  => ok(format!("Moved to {}.", to.as_str())),
        Ok(false) => err(format!(
            "Not found, not yours, or can't become {} from its current state \
             (prayer → thanksgiving; prayer/thanksgiving → released).",
            to.as_str(),
        )),
        Err(e)    => err(e.to_string()),
    }
}

/// +1 on a prayer the caller can see.
pub async fn pray_for(
    _proof: Proof<Pray>,
    user_id: UserId,
    pool: &SqlitePool,
    post_id: String,
) -> Value {
    match thanksgivings_db::repository::posts::pray(pool, &post_id, user_id).await {
        Ok(Some(c)) => ok(format!("Praying. {} so far.", tally_text(c))),
        Ok(None)    => err("Not found, not visible to you, or not a prayer (only prayers can be prayed for)."),
        Err(e)      => err(e.to_string()),
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
            format!("[{}] {} {}\n{}{}", p.id, p.state, p.created_at, p.content, history_text(p))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_transition_target_has_a_tool() {
        for to in PostState::ALL {
            let reachable = PostState::ALL.into_iter().any(|from| from.can_transition_to(to));
            assert_eq!(
                reachable,
                transition_tool(to).is_some(),
                "{to:?}: the state machine and the MCP tools disagree",
            );
        }
    }

    #[test]
    fn every_allowed_action_has_a_tool() {
        use thanksgivings_core::Viewer;
        for action in PostAction::all() {
            let ever_allowed = PostState::ALL.into_iter().any(|s| {
                [Viewer::Author, Viewer::Other].into_iter().any(|v| action.allowed(s, v))
            });
            if ever_allowed {
                assert!(action_tool(action).is_some(), "{action:?} has no MCP tool");
            }
        }
    }

    #[test]
    fn tool_names_round_trip() {
        for action in PostAction::all() {
            if let Some(tool) = action_tool(action) {
                assert_eq!(action_for_tool(tool.name()), Some(action));
            }
        }
    }
}
