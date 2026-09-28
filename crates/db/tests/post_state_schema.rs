//! The `posts.state` CHECK constraint lists states by hand in SQL, which the
//! compiler can't see. This fails when a `PostState` variant is added without
//! a migration that allows it (or when a migration drops one still in use).

use thanksgivings_core::{NewPost, OAuthProvider, PostState, Visibility};
use thanksgivings_db::{repository::{posts, users}, Db};

/// A fresh, migrated database in a temp file. (Not `sqlite::memory:` — each
/// pooled connection would get its own empty in-memory database.)
async fn fresh_db() -> (Db, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!("pray-schema-{}.db", uuid::Uuid::new_v4()));
    let db = Db::open(&format!("sqlite://{}", path.display())).await.expect("migrations run");
    (db, path)
}

#[tokio::test]
async fn schema_accepts_every_post_state() {
    let (db, path) = fresh_db().await;

    sqlx::query("INSERT INTO users (id, display_name, email, provider, provider_id) VALUES ('u', 'U', 'u@x', 'google', 'g')")
        .execute(&db.pool).await.unwrap();

    for state in PostState::ALL {
        sqlx::query("INSERT INTO posts (id, author_id, state, content, visibility) VALUES (?, 'u', ?, 'c', 'private')")
            .bind(state.as_str()).bind(state.as_str())
            .execute(&db.pool).await
            .unwrap_or_else(|e| panic!("schema rejects state {:?}: {e} — add a migration", state.as_str()));
    }

    // And the constraint still bites: an unknown state is refused.
    assert!(
        sqlx::query("INSERT INTO posts (id, author_id, state, content, visibility) VALUES ('bogus', 'u', 'bogus', 'c', 'private')")
            .execute(&db.pool).await.is_err(),
    );

    db.pool.close().await;
    let _ = std::fs::remove_file(&path);
}

/// `posts::transition` enforces `PostState::predecessors` in SQL: every
/// (from, to) pair succeeds exactly when the state machine allows it.
#[tokio::test]
async fn transitions_follow_the_state_machine() {
    let (db, path) = fresh_db().await;
    let user = users::upsert_oauth_user(&db.pool, &OAuthProvider::Google, "g1", "a@x", "A", None)
        .await.unwrap();

    for from in PostState::ALL {
        for to in PostState::ALL {
            // Reach `from` by the shortest legal route from an initial state.
            let start = if from.is_initial() { from } else { PostState::Prayer };
            let post = posts::create(&db.pool, user.id, NewPost {
                state: start, content: "c".into(), visibility: Visibility::Private,
            }).await.unwrap();
            let id = post.id.to_string();
            if start != from {
                assert!(posts::transition(&db.pool, &id, user.id, from, None).await.unwrap());
            }

            let moved = posts::transition(&db.pool, &id, user.id, to, Some("  a note  ")).await.unwrap();
            assert_eq!(moved, from.can_transition_to(to), "{from:?} → {to:?}");

            // A successful move is logged (note trimmed); a refused one isn't.
            let post = posts::fetch_by_id(&db.pool, post.id).await.unwrap().unwrap();
            assert_eq!(post.state, if moved { to } else { from });
            let last = post.history.last();
            if moved {
                let t = last.expect("move is logged");
                assert_eq!((t.from, t.to, t.note.as_deref()), (from, to, Some("a note")));
            } else {
                assert!(last.is_none_or(|t| t.to == from), "refused move must not be logged");
            }
        }
    }

    // Only the author can move their entry.
    let post = posts::create(&db.pool, user.id, NewPost {
        state: PostState::Prayer, content: "c".into(), visibility: Visibility::Private,
    }).await.unwrap();
    let stranger = uuid::Uuid::new_v4();
    assert!(!posts::transition(&db.pool, &post.id.to_string(), stranger, PostState::Released, None).await.unwrap());

    // The entry's own text is never touched by a move, and a blank note is stored as none.
    assert!(posts::transition(&db.pool, &post.id.to_string(), user.id, PostState::Thanksgiving, Some("   ")).await.unwrap());
    let post = posts::fetch_by_id(&db.pool, post.id).await.unwrap().unwrap();
    assert_eq!(post.content, "c");
    assert_eq!(post.history.len(), 1);
    assert_eq!(post.history[0].note, None);

    db.pool.close().await;
    let _ = std::fs::remove_file(&path);
}

/// `posts::pray` (+1): anyone who can see a prayer may press it as often as
/// they like; nobody else can, and only prayers (not thanksgivings) take it.
#[tokio::test]
async fn praying_now_counts_every_press_for_viewers_only() {
    use thanksgivings_core::PrayerCount;
    use thanksgivings_db::repository::groups;

    let (db, path) = fresh_db().await;
    let p = &db.pool;
    let author   = users::upsert_oauth_user(p, &OAuthProvider::Google, "a", "a@x", "A", None).await.unwrap();
    let member   = users::upsert_oauth_user(p, &OAuthProvider::Google, "m", "m@x", "M", None).await.unwrap();
    let stranger = users::upsert_oauth_user(p, &OAuthProvider::Google, "s", "s@x", "S", None).await.unwrap();
    let group = groups::create(p, "G", author.id).await.unwrap();
    groups::add_member(p, group.id, member.id).await.unwrap();

    let new = |visibility| NewPost { state: PostState::Prayer, content: "c".into(), visibility };
    let private = posts::create(p, author.id, new(Visibility::Private)).await.unwrap().id.to_string();
    let grouped = posts::create(p, author.id, new(Visibility::Group(group.id))).await.unwrap().id.to_string();
    let public  = posts::create(p, author.id, new(Visibility::Public)).await.unwrap().id.to_string();

    // Mashing: every press counts; people counts distinct users.
    for _ in 0..3 { posts::pray(p, &grouped, member.id).await.unwrap().unwrap(); }
    let tally = posts::pray(p, &grouped, author.id).await.unwrap().unwrap();
    assert_eq!(tally, PrayerCount { total: 4, people: 2 });

    // Visibility: private is author-only, group is members-only, public is anyone.
    assert!(posts::pray(p, &private, member.id).await.unwrap().is_none());
    assert!(posts::pray(p, &private, author.id).await.unwrap().is_some());
    assert!(posts::pray(p, &grouped, stranger.id).await.unwrap().is_none());
    assert!(posts::pray(p, &public, stranger.id).await.unwrap().is_some());

    // Listings carry the tally.
    let listed = posts::fetch_by_id(p, grouped.parse().unwrap()).await.unwrap().unwrap();
    assert_eq!(listed.prayers, PrayerCount { total: 4, people: 2 });

    // Only prayers take it: not once answered.
    assert!(posts::transition(p, &public, author.id, PostState::Thanksgiving, None).await.unwrap());
    assert!(posts::pray(p, &public, stranger.id).await.unwrap().is_none());

    db.pool.close().await;
    let _ = std::fs::remove_file(&path);
}
