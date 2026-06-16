use sqlx::SqlitePool;
use time::OffsetDateTime;
use uuid::Uuid;

use thanksgivings_core::{NewPost, Post, PostId, PostKind, UserId, Visibility, VisibilityFilter};

#[derive(sqlx::FromRow)]
struct PostRow {
    id: String, author_id: String, kind: String, content: String,
    visibility: String, prayer_id: Option<String>,
    created_at: String, updated_at: String,
}

impl From<PostRow> for Post {
    fn from(r: PostRow) -> Self {
        Post {
            id:        Uuid::parse_str(&r.id).unwrap(),
            author_id: Uuid::parse_str(&r.author_id).unwrap(),
            kind:      if r.kind == "prayer" { PostKind::Prayer } else { PostKind::Praise },
            content:   r.content,
            visibility: decode_visibility(&r.visibility),
            prayer_id:  r.prayer_id.as_deref().and_then(|s| Uuid::parse_str(s).ok()),
            created_at: parse_dt(&r.created_at),
            updated_at: parse_dt(&r.updated_at),
        }
    }
}

const SELECT_POST: &str =
    "SELECT id, author_id, kind, content, visibility, prayer_id, created_at, updated_at FROM posts";

pub async fn create(pool: &SqlitePool, author_id: UserId, new: NewPost) -> Result<Post, sqlx::Error> {
    let id  = Uuid::new_v4().to_string();
    let now = OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339).unwrap();
    let kind = match new.kind { PostKind::Prayer => "prayer", PostKind::Praise => "praise" };
    let vis  = encode_visibility(&new.visibility);
    let pid  = new.prayer_id.map(|u| u.to_string());

    sqlx::query(
        "INSERT INTO posts (id, author_id, kind, content, visibility, prayer_id, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id).bind(author_id.to_string()).bind(kind)
    .bind(&new.content).bind(&vis).bind(&pid).bind(&now).bind(&now)
    .execute(pool).await?;

    fetch_by_id(pool, Uuid::parse_str(&id).unwrap())
        .await?
        .ok_or(sqlx::Error::RowNotFound)
}

pub async fn fetch_by_id(pool: &SqlitePool, id: PostId) -> Result<Option<Post>, sqlx::Error> {
    Ok(
        sqlx::query_as::<_, PostRow>(&format!("{SELECT_POST} WHERE id = ?"))
            .bind(id.to_string())
            .fetch_optional(pool).await?
            .map(Post::from),
    )
}

pub async fn list_for_viewer(
    pool: &SqlitePool,
    viewer_id: UserId,
    filter: &VisibilityFilter,
) -> Result<Vec<Post>, sqlx::Error> {
    let vid = viewer_id.to_string();

    let rows: Vec<PostRow> = match filter {
        VisibilityFilter::Mine => {
            sqlx::query_as::<_, PostRow>(
                &format!("{SELECT_POST} WHERE author_id = ? AND visibility = 'private' ORDER BY created_at DESC"),
            )
            .bind(&vid)
            .fetch_all(pool).await?
        }
        VisibilityFilter::Public => {
            sqlx::query_as::<_, PostRow>(
                &format!("{SELECT_POST} WHERE visibility = 'public' ORDER BY created_at DESC"),
            )
            .fetch_all(pool).await?
        }
        VisibilityFilter::Group(gid) => {
            let gid_str = gid.to_string();
            #[derive(sqlx::FromRow)] struct Count { cnt: i64 }
            let ok = sqlx::query_as::<_, Count>(
                "SELECT COUNT(*) as cnt FROM memberships WHERE user_id = ? AND group_id = ?",
            )
            .bind(&vid).bind(&gid_str)
            .fetch_one(pool).await?.cnt > 0;

            if !ok { return Ok(vec![]); }

            sqlx::query_as::<_, PostRow>(
                &format!("{SELECT_POST} WHERE visibility = ? ORDER BY created_at DESC"),
            )
            .bind(&gid_str)
            .fetch_all(pool).await?
        }
    };

    Ok(rows.into_iter().map(Post::from).collect())
}

/// All public prayers from any author.
pub async fn list_public(pool: &SqlitePool) -> Result<Vec<Post>, sqlx::Error> {
    let rows: Vec<PostRow> = sqlx::query_as::<_, PostRow>(
        &format!("{SELECT_POST} WHERE visibility = 'public' ORDER BY created_at DESC"),
    )
    .fetch_all(pool).await?;
    Ok(rows.into_iter().map(Post::from).collect())
}

/// All prayers by a given author regardless of visibility.
pub async fn list_all_for_author(pool: &SqlitePool, author_id: UserId) -> Result<Vec<Post>, sqlx::Error> {
    let rows: Vec<PostRow> = sqlx::query_as::<_, PostRow>(
        &format!("{SELECT_POST} WHERE author_id = ? ORDER BY created_at DESC"),
    )
    .bind(author_id.to_string())
    .fetch_all(pool).await?;
    Ok(rows.into_iter().map(Post::from).collect())
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

pub async fn delete(
    pool: &SqlitePool,
    post_id: &str,
    author_id: UserId,
) -> Result<bool, sqlx::Error> {
    let uid = author_id.to_string();
    let result = sqlx::query("DELETE FROM posts WHERE id = ? AND author_id = ?")
        .bind(post_id).bind(&uid)
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
    let result = sqlx::query(
        "UPDATE posts SET content = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')
         WHERE id = ? AND author_id = ?",
    )
    .bind(content).bind(post_id).bind(&uid)
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
