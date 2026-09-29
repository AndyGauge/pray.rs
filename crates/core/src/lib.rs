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

/// Lifecycle of an entry. A prayer may become a thanksgiving once answered,
/// and either may be released. An entry can also be written as a thanksgiving
/// from the start. Released is terminal and hidden from every listing.
///
/// ```text
/// Prayer ──► Thanksgiving
///    │            │
///    └──► Released ◄┘
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostState {
    Prayer,
    Thanksgiving,
    Released,
}

impl PostState {
    /// Every state, for UIs that iterate over the machine (see `_ALL_IS_COMPLETE`).
    pub const ALL: [PostState; 3] = [PostState::Prayer, PostState::Thanksgiving, PostState::Released];

    /// States a new entry may be created in.
    pub fn is_initial(self) -> bool {
        match self {
            PostState::Prayer | PostState::Thanksgiving => true,
            PostState::Released                         => false,
        }
    }

    /// States an entry must currently be in to move into `self`.
    pub fn predecessors(self) -> &'static [PostState] {
        match self {
            PostState::Prayer       => &[],
            PostState::Thanksgiving => &[PostState::Prayer],
            PostState::Released     => &[PostState::Prayer, PostState::Thanksgiving],
        }
    }

    pub fn can_transition_to(self, to: PostState) -> bool {
        to.predecessors().contains(&self)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            PostState::Prayer       => "prayer",
            PostState::Thanksgiving => "thanksgiving",
            PostState::Released     => "released",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "prayer"       => Some(PostState::Prayer),
            "thanksgiving" => Some(PostState::Thanksgiving),
            "released"     => Some(PostState::Released),
            _              => None,
        }
    }
}

// Adding a variant breaks this match. When it does, add the variant to
// `PostState::ALL` too, otherwise UIs that iterate `ALL` will never offer it.
const _ALL_IS_COMPLETE: () = {
    const fn check(s: PostState) {
        match s {
            PostState::Prayer | PostState::Thanksgiving | PostState::Released => {}
        }
    }
    let _ = check;
};

impl std::fmt::Display for PostState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PostState::Prayer       => write!(f, "Prayer"),
            PostState::Thanksgiving => write!(f, "Thanksgiving"),
            PostState::Released     => write!(f, "Released"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: PostId,
    pub author_id: UserId,
    pub state: PostState,
    pub content: String,
    pub visibility: Visibility,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    /// Lifecycle moves so far, oldest first. `content` is never rewritten by a
    /// transition; notes recorded here are shown after it.
    pub history: Vec<PostTransition>,
    /// "Praying now" presses on this entry.
    pub prayers: PrayerCount,
}

/// Tally of "praying now" presses. Every press counts, so one person can
/// pray for an entry again and again.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrayerCount {
    /// Total presses by everyone.
    pub total: i64,
    /// Distinct people who have pressed.
    pub people: i64,
}

// ─── Actions ─────────────────────────────────────────────────────────────────

/// How the person looking at an entry relates to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Viewer {
    /// Wrote it.
    Author,
    /// Anyone else who can see it (a group member, or anyone on a public entry).
    Other,
}

/// Everything a person can do to an entry. The UI and MCP derive their
/// buttons and tools from this, with exhaustive `match`es, so a new action
/// won't compile until both expose it (or deliberately don't).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "to")]
pub enum PostAction {
    /// Move the entry along its lifecycle (author only).
    MoveTo(PostState),
    /// "I'm praying for this now" (+1). Anyone who can see a prayer; repeatable.
    PrayingNow,
}

impl PostAction {
    /// Every action, for UIs that iterate them. A new variant breaks the
    /// `match` below; add it to the returned list too.
    pub fn all() -> impl Iterator<Item = PostAction> {
        const fn _complete(a: PostAction) {
            match a {
                PostAction::MoveTo(_) | PostAction::PrayingNow => {}
            }
        }
        // Order is display order: +1 first, then lifecycle moves.
        [PostAction::PrayingNow].into_iter().chain(PostState::ALL.into_iter().map(PostAction::MoveTo))
    }

    /// Whether `viewer` may take this action on an entry in `state`.
    /// The single source of truth for UI, MCP and the server.
    pub fn allowed(self, state: PostState, viewer: Viewer) -> bool {
        match self {
            PostAction::MoveTo(to) => viewer == Viewer::Author && state.can_transition_to(to),
            PostAction::PrayingNow => match state {
                PostState::Prayer                              => true,
                PostState::Thanksgiving | PostState::Released => false,
            },
        }
    }
}

impl Viewer {
    /// How `user` relates to `post`. The one definition of "author" vs "other";
    /// whether `user` may see the post at all is checked separately (by the
    /// query that loaded it, or `posts::visible_to`).
    pub fn of(post: &Post, user: UserId) -> Viewer {
        if post.author_id == user { Viewer::Author } else { Viewer::Other }
    }
}

/// A post as seen by a particular person, so the UI knows which actions to offer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewedPost {
    pub post: Post,
    pub viewer: Viewer,
}

impl ViewedPost {
    pub fn new(post: Post, user: UserId) -> Self {
        let viewer = Viewer::of(&post, user);
        ViewedPost { post, viewer }
    }
}

/// One recorded lifecycle move, with the author's optional note about it
/// (e.g. how a prayer was answered).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostTransition {
    pub from: PostState,
    pub to: PostState,
    pub note: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPost {
    /// Must be an initial state (Prayer or Thanksgiving).
    pub state: PostState,
    pub content: String,
    pub visibility: Visibility,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn post_by(author: UserId) -> Post {
        Post {
            id: Uuid::new_v4(), author_id: author, state: PostState::Prayer,
            content: String::new(), visibility: Visibility::Private,
            created_at: OffsetDateTime::UNIX_EPOCH, updated_at: OffsetDateTime::UNIX_EPOCH,
            history: vec![], prayers: PrayerCount::default(),
        }
    }

    #[test]
    fn viewer_of_is_author_only_for_the_author() {
        let (me, you) = (Uuid::new_v4(), Uuid::new_v4());
        assert_eq!(Viewer::of(&post_by(me), me), Viewer::Author);
        assert_eq!(Viewer::of(&post_by(me), you), Viewer::Other);
        assert_eq!(ViewedPost::new(post_by(me), you).viewer, Viewer::Other);
    }
}
