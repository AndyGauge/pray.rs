//! Accounts are per provider: the same email may sign in with Google and with
//! Facebook as two separate accounts (migration 009).

use thanksgivings_core::OAuthProvider;
use thanksgivings_db::{repository::users, Db};

async fn fresh_db() -> (Db, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!("pray-users-{}.db", uuid::Uuid::new_v4()));
    let db = Db::open(&format!("sqlite://{}", path.display())).await.expect("migrations run");
    (db, path)
}

#[tokio::test]
async fn same_email_on_two_providers_is_two_accounts() {
    let (db, path) = fresh_db().await;
    let p = &db.pool;

    let google = users::upsert_oauth_user(p, &OAuthProvider::Google, "g-1", "me@x", "Me", None).await.unwrap();
    let facebook = users::upsert_oauth_user(p, &OAuthProvider::Facebook, "f-1", "me@x", "Me", None).await.unwrap();
    assert_ne!(google.id, facebook.id);

    // Signing in again with the same provider identity returns the same account.
    let again = users::upsert_oauth_user(p, &OAuthProvider::Google, "g-1", "me@x", "Me (renamed)", None).await.unwrap();
    assert_eq!(again.id, google.id);
    assert_eq!(again.display_name, "Me (renamed)");

    // Within one provider an email still belongs to one account.
    assert!(users::upsert_oauth_user(p, &OAuthProvider::Google, "g-2", "me@x", "Other", None).await.is_err());

    db.pool.close().await;
    let _ = std::fs::remove_file(&path);
}

/// Migrations run with foreign keys off (see `Db::open`); the pool must still
/// enforce them afterwards.
#[tokio::test]
async fn foreign_keys_are_enforced_after_migrating() {
    let (db, path) = fresh_db().await;
    let insert_orphan = sqlx::query(
        "INSERT INTO posts (id, author_id, state, content, visibility) VALUES ('p', 'no-such-user', 'prayer', 'c', 'private')",
    )
    .execute(&db.pool)
    .await;
    assert!(insert_orphan.is_err(), "a post with a missing author must be rejected");

    db.pool.close().await;
    let _ = std::fs::remove_file(&path);
}
