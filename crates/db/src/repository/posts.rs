use sqlx::SqlitePool;
use time::OffsetDateTime;
use uuid::Uuid;

use std::collections::HashMap;

use thanksgivings_core::{
    NewPost, Post, PostAction, PostId, PostState, PostTransition, PrayerCount, UserId, Viewer,
    Visibility, VisibilityFilter,
};

struct PostRow {
    id: String, author_id: String, state: String, content: String,
    visibility: String,
    created_at: String, updated_at: String,
}

impl From<PostRow> for Post {
    fn from(r: PostRow) -> Self {
        Post {
            id:        Uuid::parse_str(&r.id).unwrap(),
            author_id: Uuid::parse_str(&r.author_id).unwrap(),
            state:     PostState::parse(&r.state).unwrap_or(PostState::Prayer),
            content:   r.content,
            visibility: decode_visibility(&r.visibility),
            created_at: parse_dt(&r.created_at),
            updated_at: parse_dt(&r.updated_at),
            history:    Vec::new(),            // filled by `with_history`
            prayers:    PrayerCount::default(), // likewise
        }
    }
}

/// Convert rows to posts, loading every post's transition log and prayer
/// tally (one query each).
async fn with_history(pool: &SqlitePool, rows: Vec<PostRow>) -> Result<Vec<Post>, sqlx::Error> {
    if rows.is_empty() { return Ok(vec![]); }
    let ids_json = serde_json::to_string(&rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>())
        .expect("serialize post ids");
    let log = sqlx::query!(
        "SELECT post_id, from_state, to_state, note, created_at
         FROM post_transitions
         WHERE post_id IN (SELECT value FROM json_each(?))
         ORDER BY created_at, rowid",
        ids_json,
    )
    .fetch_all(pool).await?;

    let mut by_post: HashMap<String, Vec<PostTransition>> = HashMap::new();
    for t in log {
        let (Some(from), Some(to)) = (PostState::parse(&t.from_state), PostState::parse(&t.to_state)) else {
            continue;
        };
        by_post.entry(t.post_id).or_default().push(PostTransition {
            from, to, note: t.note, at: parse_dt(&t.created_at),
        });
    }

    let tallies: HashMap<String, PrayerCount> = sqlx::query!(
        r#"SELECT post_id, SUM(count) AS "total!: i64", COUNT(*) AS "people!: i64"
           FROM post_prayers
           WHERE post_id IN (SELECT value FROM json_each(?))
           GROUP BY post_id"#,
        ids_json,
    )
    .fetch_all(pool).await?
    .into_iter()
    .map(|t| (t.post_id, PrayerCount { total: t.total, people: t.people }))
    .collect();

    Ok(rows.into_iter().map(|r| {
        let history = by_post.remove(&r.id).unwrap_or_default();
        let prayers = tallies.get(&r.id).copied().unwrap_or_default();
        Post { history, prayers, ..Post::from(r) }
    }).collect())
}

// The query macros need literal SQL, so each SELECT spells out its columns.
// Every listing filters `state != 'released'`: released entries are hidden.

pub async fn create(pool: &SqlitePool, author_id: UserId, new: NewPost) -> Result<Post, sqlx::Error> {
    let id  = Uuid::new_v4().to_string();
    let uid = author_id.to_string();
    let now = OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339).unwrap();
    // Only Prayer/Thanksgiving are valid starting states; never create released.
    let state = if new.state.is_initial() { new.state } else { PostState::Prayer }.as_str();
    let vis   = encode_visibility(&new.visibility);

    sqlx::query!(
        "INSERT INTO posts (id, author_id, state, content, visibility, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        id, uid, state, new.content, vis, now, now,
    )
    .execute(pool).await?;

    fetch_by_id(pool, Uuid::parse_str(&id).unwrap())
        .await?
        .ok_or(sqlx::Error::RowNotFound)
}

pub async fn fetch_by_id(pool: &SqlitePool, id: PostId) -> Result<Option<Post>, sqlx::Error> {
    let id = id.to_string();
    let row = sqlx::query_as!(
        PostRow,
        r#"SELECT id AS "id!", author_id, state, content, visibility, created_at, updated_at
           FROM posts WHERE id = ?"#,
        id,
    )
    .fetch_optional(pool).await?;
    Ok(with_history(pool, row.into_iter().collect()).await?.pop())
}

pub async fn list_for_viewer(
    pool: &SqlitePool,
    viewer_id: UserId,
    filter: &VisibilityFilter,
) -> Result<Vec<Post>, sqlx::Error> {
    let vid = viewer_id.to_string();

    let rows: Vec<PostRow> = match filter {
        VisibilityFilter::Mine => {
            sqlx::query_as!(
                PostRow,
                r#"SELECT id AS "id!", author_id, state, content, visibility, created_at, updated_at
                   FROM posts
                   WHERE author_id = ? AND visibility = 'private' AND state != 'released'
                   ORDER BY created_at DESC"#,
                vid,
            )
            .fetch_all(pool).await?
        }
        VisibilityFilter::Public => return list_public(pool).await,
        VisibilityFilter::Group(gid) => {
            let gid_str = gid.to_string();
            let ok = sqlx::query!(
                "SELECT COUNT(*) AS cnt FROM memberships WHERE user_id = ? AND group_id = ?",
                vid, gid_str,
            )
            .fetch_one(pool).await?.cnt > 0;

            if !ok { return Ok(vec![]); }

            sqlx::query_as!(
                PostRow,
                r#"SELECT id AS "id!", author_id, state, content, visibility, created_at, updated_at
                   FROM posts
                   WHERE visibility = ? AND state != 'released'
                   ORDER BY created_at DESC"#,
                gid_str,
            )
            .fetch_all(pool).await?
        }
    };

    with_history(pool, rows).await
}

/// All public prayers from any author.
pub async fn list_public(pool: &SqlitePool) -> Result<Vec<Post>, sqlx::Error> {
    let rows = sqlx::query_as!(
        PostRow,
        r#"SELECT id AS "id!", author_id, state, content, visibility, created_at, updated_at
           FROM posts
           WHERE visibility = 'public' AND state != 'released'
           ORDER BY created_at DESC"#,
    )
    .fetch_all(pool).await?;
    with_history(pool, rows).await
}

/// All non-released prayers by a given author regardless of visibility.
pub async fn list_all_for_author(pool: &SqlitePool, author_id: UserId) -> Result<Vec<Post>, sqlx::Error> {
    let uid = author_id.to_string();
    let rows = sqlx::query_as!(
        PostRow,
        r#"SELECT id AS "id!", author_id, state, content, visibility, created_at, updated_at
           FROM posts
           WHERE author_id = ? AND state != 'released'
           ORDER BY created_at DESC"#,
        uid,
    )
    .fetch_all(pool).await?;
    with_history(pool, rows).await
}

/// Change visibility of a post — only succeeds if the caller is the author.
pub async fn set_visibility(
    pool: &SqlitePool,
    post_id: &str,
    author_id: UserId,
    visibility: &str,
) -> Result<bool, sqlx::Error> {
    let uid = author_id.to_string();
    let result = sqlx::query!(
        "UPDATE posts SET visibility = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')
         WHERE id = ? AND author_id = ?",
        visibility, post_id, uid,
    )
    .execute(pool).await?;
    Ok(result.rows_affected() > 0)
}

/// Move one of the author's entries to `to`, recording the move (and the
/// author's optional `note` about it) in `post_transitions`. The state change
/// and the log entry commit together.
///
/// Returns false if the post isn't theirs or the state machine doesn't allow
/// `to` from its current state. The UPDATE is conditioned on the state read at
/// the start, so a concurrent move makes this one match no row.
pub async fn transition(
    pool: &SqlitePool,
    post_id: &str,
    author_id: UserId,
    to: PostState,
    note: Option<&str>,
) -> Result<bool, sqlx::Error> {
    let uid  = author_id.to_string();
    let note = note.map(str::trim).filter(|n| !n.is_empty());
    let mut tx = pool.begin().await?;

    let Some(row) = sqlx::query!("SELECT state FROM posts WHERE id = ? AND author_id = ?", post_id, uid)
        .fetch_optional(&mut *tx).await?
    else { return Ok(false) };
    let Some(from) = PostState::parse(&row.state) else { return Ok(false) };
    if !from.can_transition_to(to) { return Ok(false); }

    let (from, to) = (from.as_str(), to.as_str());
    let updated = sqlx::query!(
        "UPDATE posts SET state = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')
         WHERE id = ? AND author_id = ? AND state = ?",
        to, post_id, uid, from,
    )
    .execute(&mut *tx).await?;
    if updated.rows_affected() == 0 { return Ok(false); }

    let log_id = Uuid::new_v4().to_string();
    sqlx::query!(
        "INSERT INTO post_transitions (id, post_id, from_state, to_state, note) VALUES (?, ?, ?, ?, ?)",
        log_id, post_id, from, to, note,
    )
    .execute(&mut *tx).await?;

    tx.commit().await?;
    Ok(true)
}

/// Record that `user_id` is praying for a post right now (+1). Every call
/// counts, so pressing again and again keeps adding.
///
/// Returns the post's new tally, or `None` if the user can't see the post or
/// `PostAction::PrayingNow` isn't allowed in its state (e.g. a thanksgiving).
pub async fn pray(
    pool: &SqlitePool,
    post_id: &str,
    user_id: UserId,
) -> Result<Option<PrayerCount>, sqlx::Error> {
    let uid = user_id.to_string();
    let mut tx = pool.begin().await?;

    let Some(post) = sqlx::query!(
        "SELECT author_id, state, visibility FROM posts WHERE id = ?",
        post_id,
    )
    .fetch_optional(&mut *tx).await?
    else { return Ok(None) };

    let viewer = if post.author_id == uid { Viewer::Author } else { Viewer::Other };
    // Private: author only. Public: anyone. Otherwise a group id: members only.
    let can_see = viewer == Viewer::Author
        || post.visibility == "public"
        || (post.visibility != "private"
            && sqlx::query!(
                "SELECT COUNT(*) AS cnt FROM memberships WHERE user_id = ? AND group_id = ?",
                uid, post.visibility,
            )
            .fetch_one(&mut *tx).await?.cnt > 0);
    let Some(state) = PostState::parse(&post.state) else { return Ok(None) };
    if !can_see || !PostAction::PrayingNow.allowed(state, viewer) {
        return Ok(None);
    }

    sqlx::query!(
        "INSERT INTO post_prayers (post_id, user_id) VALUES (?, ?)
         ON CONFLICT (post_id, user_id) DO UPDATE SET
             count          = count + 1,
             last_prayed_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')",
        post_id, uid,
    )
    .execute(&mut *tx).await?;

    let t = sqlx::query!(
        r#"SELECT COALESCE(SUM(count), 0) AS "total!: i64", COUNT(*) AS "people!: i64"
           FROM post_prayers WHERE post_id = ?"#,
        post_id,
    )
    .fetch_one(&mut *tx).await?;

    tx.commit().await?;
    Ok(Some(PrayerCount { total: t.total, people: t.people }))
}

pub async fn delete(
    pool: &SqlitePool,
    post_id: &str,
    author_id: UserId,
) -> Result<bool, sqlx::Error> {
    let uid = author_id.to_string();
    let result = sqlx::query!("DELETE FROM posts WHERE id = ? AND author_id = ?", post_id, uid)
        .execute(pool).await?;
    Ok(result.rows_affected() > 0)
}

pub async fn update_content(
    pool: &SqlitePool,
    post_id: &str,
    author_id: UserId,
    content: &str,
) -> Result<bool, sqlx::Error> {
    let uid = author_id.to_string();
    let result = sqlx::query!(
        "UPDATE posts SET content = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')
         WHERE id = ? AND author_id = ?",
        content, post_id, uid,
    )
    .execute(pool).await?;
    Ok(result.rows_affected() > 0)
}

fn encode_visibility(v: &Visibility) -> String {
    match v {
        Visibility::Private    => "private".to_string(),
        Visibility::Public     => "public".to_string(),
        Visibility::Group(id)  => id.to_string(),
    }
}

fn decode_visibility(s: &str) -> Visibility {
    match s {
        "private" => Visibility::Private,
        "public"  => Visibility::Public,
        other     => Visibility::Group(Uuid::parse_str(other).unwrap_or_default()),
    }
}

fn parse_dt(s: &str) -> OffsetDateTime {
    OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339)
        .unwrap_or(OffsetDateTime::UNIX_EPOCH)
}
