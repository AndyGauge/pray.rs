use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

pub type UserId  = Uuid;
pub type PostId  = Uuid;
pub type GroupId = Uuid;

// ─── Visibility ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "id")]
pub enum Visibility {
    Private,
    Group(GroupId),
    Public,
}

/// What the UI picker uses to choose which posts to page through.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum VisibilityFilter {
    #[default]
    Mine,               // private posts authored by me
    Group(GroupId),     // posts shared to a group I'm in
    Public,             // public posts
}

// ─── Posts ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostKind {
    Prayer,
    Praise,
}

impl std::fmt::Display for PostKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PostKind::Prayer => write!(f, "Prayer"),
            PostKind::Praise => write!(f, "Praise"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: PostId,
    pub author_id: UserId,
    pub kind: PostKind,
    pub content: String,
    pub visibility: Visibility,
    /// If this is a praise, the prayer it answers.
    pub prayer_id: Option<PostId>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPost {
    pub kind: PostKind,
    pub content: String,
    pub visibility: Visibility,
    pub prayer_id: Option<PostId>,
}

// ─── Users ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OAuthProvider {
    Google,
    Facebook,
}

impl std::fmt::Display for OAuthProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OAuthProvider::Google   => write!(f, "google"),
            OAuthProvider::Facebook => write!(f, "facebook"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub display_name: String,
    pub email: String,
    pub avatar_url: Option<String>,
    pub provider: OAuthProvider,
    pub provider_id: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

// ─── Groups ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: GroupId,
    pub name: String,
    pub owner_id: UserId,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Membership {
    pub user_id: UserId,
    pub group_id: GroupId,
    #[serde(with = "time::serde::rfc3339")]
    pub joined_at: OffsetDateTime,
}

// ─── Invitations ─────────────────────────────────────────────────────────────

pub type InvitationId = Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactType { Email, Phone }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvitationStatus { Pending, Accepted, Declined }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invitation {
    pub id: InvitationId,
    pub group_id: GroupId,
    pub invited_by: UserId,
    pub contact: String,
    pub contact_type: ContactType,
    pub token: String,
    pub status: InvitationStatus,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: OffsetDateTime,
}

/// Lightweight summary for displaying pending invites in the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteSummary {
    pub id: InvitationId,
    pub contact: String,
    pub contact_type: ContactType,
    pub invited_by_name: String,
    pub group_name: String,
    pub token: String,
}
