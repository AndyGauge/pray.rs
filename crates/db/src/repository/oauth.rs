use sqlx::SqlitePool;
use uuid::Uuid;
use thanksgivings_core::UserId;

// ── Hashing ────────────────────────────────────────────────────────────────────
// Codes and tokens are stored only as SHA-256 hex, never in the clear.

pub fn hash(raw: &str) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(raw.as_bytes()))
}

// ── Clients (RFC 7591 Dynamic Client Registration) ──────────────────────────────

#[derive(Debug, Clone)]
pub struct Client {
    pub client_id:     String,
    pub client_name:   Option<String>,
    pub redirect_uris: Vec<String>,
}

/// Register a new public client. `redirect_uris` is stored as a JSON array.
pub async fn register_client(
    pool: &SqlitePool,
    client_name: Option<&str>,
    redirect_uris: &[String],
) -> Result<Client, sqlx::Error> {
    let client_id = format!("mcp_{}", Uuid::new_v4().simple());
    let uris_json = serde_json::to_string(redirect_uris).unwrap_or_else(|_| "[]".into());

    sqlx::query("INSERT INTO oauth_clients (client_id, client_name, redirect_uris) VALUES (?, ?, ?)")
        .bind(&client_id)
        .bind(client_name)
        .bind(&uris_json)
        .execute(pool)
        .await?;

    Ok(Client {
        client_id,
        client_name: client_name.map(str::to_string),
        redirect_uris: redirect_uris.to_vec(),
    })
}

pub async fn get_client(pool: &SqlitePool, client_id: &str)
    -> Result<Option<Client>, sqlx::Error>
{
    let row = sqlx::query("SELECT client_id, client_name, redirect_uris FROM oauth_clients WHERE client_id = ?")
        .bind(client_id)
        .fetch_optional(pool)
        .await?;

    let Some(row) = row else { return Ok(None) };
    use sqlx::Row;
    let uris_json: String = row.try_get("redirect_uris")?;
    Ok(Some(Client {
        client_id:     row.try_get("client_id")?,
        client_name:   row.try_get("client_name")?,
        redirect_uris: serde_json::from_str(&uris_json).unwrap_or_default(),
    }))
}

// ── Authorization codes (PKCE) ──────────────────────────────────────────────────

/// Issued authorization-code grant. Returns the raw code (shown once, to the client).
pub async fn create_auth_code(
    pool: &SqlitePool,
    client_id: &str,
    user_id: UserId,
    redirect_uri: &str,
    code_challenge: &str,
    scope: Option<&str>,
    resource: Option<&str>,
    ttl_secs: i64,
) -> Result<String, sqlx::Error> {
    let raw = format!("code_{}", Uuid::new_v4().simple());
    let code_hash = hash(&raw);
    let expires_at = format!(
        "{}",
        (time::OffsetDateTime::now_utc() + time::Duration::seconds(ttl_secs))
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default()
    );

    sqlx::query(
        "INSERT INTO oauth_auth_codes \
         (code_hash, client_id, user_id, redirect_uri, code_challenge, scope, resource, expires_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&code_hash)
    .bind(client_id)
    .bind(user_id.to_string())
    .bind(redirect_uri)
    .bind(code_challenge)
    .bind(scope)
    .bind(resource)
    .bind(&expires_at)
    .execute(pool)
    .await?;

    Ok(raw)
}

pub struct AuthCode {
    pub client_id:      String,
    pub user_id:        UserId,
    pub redirect_uri:   String,
    pub code_challenge: String,
    pub scope:          Option<String>,
}

/// Atomically consume a code: returns its data only if it exists, is unused, and
/// is unexpired — and marks it used so it can never be replayed.
pub async fn consume_auth_code(pool: &SqlitePool, raw_code: &str)
    -> Result<Option<AuthCode>, sqlx::Error>
{
    let code_hash = hash(raw_code);
    let mut tx = pool.begin().await?;

    use sqlx::Row;
    let row = sqlx::query(
        "SELECT client_id, user_id, redirect_uri, code_challenge, scope \
         FROM oauth_auth_codes \
         WHERE code_hash = ? AND used = 0 \
           AND expires_at > strftime('%Y-%m-%dT%H:%M:%SZ', 'now')",
    )
    .bind(&code_hash)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(row) = row else { tx.rollback().await?; return Ok(None) };

    sqlx::query("UPDATE oauth_auth_codes SET used = 1 WHERE code_hash = ?")
        .bind(&code_hash)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    let uid: String = row.try_get("user_id")?;
    Ok(Some(AuthCode {
        client_id:      row.try_get("client_id")?,
        user_id:        Uuid::parse_str(&uid).unwrap_or_default(),
        redirect_uri:   row.try_get("redirect_uri")?,
        code_challenge: row.try_get("code_challenge")?,
        scope:          row.try_get("scope")?,
    }))
}

// ── Access + refresh tokens ──────────────────────────────────────────────────────

pub struct IssuedTokens {
    pub access_token:  String,
    pub refresh_token: String,
    pub expires_in:    i64,
}

pub async fn issue_tokens(
    pool: &SqlitePool,
    client_id: &str,
    user_id: UserId,
    scope: Option<&str>,
    ttl_secs: i64,
) -> Result<IssuedTokens, sqlx::Error> {
    let access  = format!("at_{}", Uuid::new_v4().simple());
    let refresh = format!("rt_{}", Uuid::new_v4().simple());
    let expires_at = format!(
        "{}",
        (time::OffsetDateTime::now_utc() + time::Duration::seconds(ttl_secs))
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default()
    );

    sqlx::query(
        "INSERT INTO oauth_access_tokens \
         (token_hash, refresh_hash, client_id, user_id, scope, expires_at) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(hash(&access))
    .bind(hash(&refresh))
    .bind(client_id)
    .bind(user_id.to_string())
    .bind(scope)
    .bind(&expires_at)
    .execute(pool)
    .await?;

    Ok(IssuedTokens { access_token: access, refresh_token: refresh, expires_in: ttl_secs })
}

/// Resolve a Bearer access token to its user, if valid and unexpired.
pub async fn authenticate(pool: &SqlitePool, raw_token: &str)
    -> Result<Option<UserId>, sqlx::Error>
{
    use sqlx::Row;
    let row = sqlx::query(
        "SELECT user_id FROM oauth_access_tokens \
         WHERE token_hash = ? \
           AND expires_at > strftime('%Y-%m-%dT%H:%M:%SZ', 'now')",
    )
    .bind(hash(raw_token))
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else { return Ok(None) };
    let uid: String = row.try_get("user_id")?;
    Ok(Uuid::parse_str(&uid).ok())
}

pub struct RefreshContext {
    pub client_id: String,
    pub user_id:   UserId,
    pub scope:     Option<String>,
}

/// Consume a refresh token (rotating it out) and return what's needed to mint a
/// fresh pair. Deletes the old row so the refresh token is single-use.
pub async fn consume_refresh(pool: &SqlitePool, raw_refresh: &str)
    -> Result<Option<RefreshContext>, sqlx::Error>
{
    use sqlx::Row;
    let refresh_hash = hash(raw_refresh);
    let mut tx = pool.begin().await?;

    let row = sqlx::query(
        "SELECT client_id, user_id, scope FROM oauth_access_tokens WHERE refresh_hash = ?",
    )
    .bind(&refresh_hash)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(row) = row else { tx.rollback().await?; return Ok(None) };

    sqlx::query("DELETE FROM oauth_access_tokens WHERE refresh_hash = ?")
        .bind(&refresh_hash)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    let uid: String = row.try_get("user_id")?;
    Ok(Some(RefreshContext {
        client_id: row.try_get("client_id")?,
        user_id:   Uuid::parse_str(&uid).unwrap_or_default(),
        scope:     row.try_get("scope")?,
    }))
}
