pub mod repository;

use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("migrate: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("foreign key check failed after migrating: {0} dangling reference(s)")]
    ForeignKeys(usize),
}

#[derive(Clone)]
pub struct Db {
    pub pool: SqlitePool,
}

impl Db {
    pub async fn open(url: &str) -> Result<Self, DbError> {
        let opts = SqliteConnectOptions::from_str(url)?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(opts)
            .await?;

        migrate(&pool).await?;

        Ok(Self { pool })
    }
}

/// Run migrations with foreign keys OFF, then verify nothing dangles.
///
/// Table rebuilds (SQLite's only way to change a constraint) drop the old
/// table; with foreign keys on, that drop cascades and deletes every row that
/// references it. SQLite only honours `PRAGMA foreign_keys` outside a
/// transaction, and sqlx-sqlite runs each migration inside one (it ignores
/// `-- no-transaction`), so the pragma is set here, on the one connection the
/// migrations run on, before any transaction starts. The connection is closed
/// afterwards so the pool never hands out a connection with checks disabled.
async fn migrate(pool: &SqlitePool) -> Result<(), DbError> {
    let mut conn = pool.acquire().await?;
    sqlx::query("PRAGMA foreign_keys = OFF").execute(&mut *conn).await?;

    let result = sqlx::migrate!("./migrations").run(&mut *conn).await;
    let dangling = sqlx::query("PRAGMA foreign_key_check").fetch_all(&mut *conn).await;

    conn.close_on_drop();
    result?;
    match dangling?.len() {
        0 => Ok(()),
        n => Err(DbError::ForeignKeys(n)),
    }
}
