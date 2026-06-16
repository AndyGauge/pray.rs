//! OAuth 2.1 authorization server for the MCP endpoint.
//!
//! Implements just enough of the MCP authorization spec for claude.ai (a public
//! PKCE client) to connect with no pre-shared secret:
//!   - RFC 9728 protected-resource metadata
//!   - RFC 8414 authorization-server metadata
//!   - RFC 7591 dynamic client registration
//!   - authorization-code grant with PKCE (S256)
//!
//! Identity is delegated to the existing Google/Facebook login: `/authorize`
//! bounces an unauthenticated browser through `/login` and back.

use axum::{
    extract::{Extension, Query, RawQuery},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    Form, Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use tower_sessions::Session;

use crate::server::AppState;
use thanksgivings_core::UserId;
use thanksgivings_db::repository::oauth;

const CODE_TTL_SECS: i64 = 600;        // authorization codes: 10 min
const ACCESS_TTL_SECS: i64 = 3600;     // access tokens: 1 hour

fn base_url() -> String {
    std::env::var("BASE_URL").unwrap_or_else(|_| "https://pray.rs".to_string())
}

// ── Discovery metadata ──────────────────────────────────────────────────────────

/// RFC 9728 — tells the client which authorization server protects `/mcp`.
pub async fn protected_resource_metadata() -> Response {
    let base = base_url();
    Json(json!({
        "resource": format!("{base}/mcp"),
        "authorization_servers": [base],
    })).into_response()
}

/// RFC 8414 — authorization server metadata (endpoints + capabilities).
pub async fn authorization_server_metadata() -> Response {
    let base = base_url();
    Json(json!({
        "issuer": base,
        "authorization_endpoint": format!("{base}/authorize"),
        "token_endpoint":         format!("{base}/token"),
        "registration_endpoint":  format!("{base}/register"),
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "code_challenge_methods_supported": ["S256"],
        "token_endpoint_auth_methods_supported": ["none"],
        "scopes_supported": ["mcp"],
    })).into_response()
}

// ── Dynamic client registration (RFC 7591) ──────────────────────────────────────

pub async fn register(
    Extension(state): Extension<AppState>,
    Json(body): Json<Value>,
) -> Response {
    let redirect_uris: Vec<String> = body["redirect_uris"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default();

    if redirect_uris.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({
            "error": "invalid_redirect_uri",
            "error_description": "at least one redirect_uri is required",
        }))).into_response();
    }

    let client_name = body["client_name"].as_str();

    match oauth::register_client(&state.db.pool, client_name, &redirect_uris).await {
        Ok(client) => (StatusCode::CREATED, Json(json!({
            "client_id": client.client_id,
            "client_name": client.client_name,
            "redirect_uris": client.redirect_uris,
            "grant_types": ["authorization_code", "refresh_token"],
            "response_types": ["code"],
            "token_endpoint_auth_method": "none",
        }))).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({
            "error": "server_error",
        }))).into_response(),
    }
}

// ── Authorization endpoint ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AuthorizeParams {
    pub response_type: String,
    pub client_id: String,
    pub redirect_uri: String,
    #[serde(default)]
    pub code_challenge: String,
    #[serde(default)]
    pub code_challenge_method: String,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub resource: Option<String>,
}

/// Append `query` to `base` with the right separator, leaving any existing query intact.
fn with_query(base: &str, query: &str) -> String {
    let sep = if base.contains('?') { '&' } else { '?' };
    format!("{base}{sep}{query}")
}

/// Redirect back to the client with an OAuth error (RFC 6749 §4.1.2.1).
fn authorize_error(redirect_uri: &str, state: &Option<String>, error: &str) -> Response {
    let mut q = format!("error={}", urlencoding::encode(error));
    if let Some(s) = state {
        q.push_str(&format!("&state={}", urlencoding::encode(s)));
    }
    Redirect::to(&with_query(redirect_uri, &q)).into_response()
}

pub async fn authorize(
    Extension(state): Extension<AppState>,
    session: Session,
    RawQuery(raw): RawQuery,
    Query(params): Query<AuthorizeParams>,
) -> Response {
    // Validate the client and redirect_uri before trusting the redirect target.
    let client = match oauth::get_client(&state.db.pool, &params.client_id).await {
        Ok(Some(c)) => c,
        _ => return (StatusCode::BAD_REQUEST, "unknown client_id").into_response(),
    };
    if !client.redirect_uris.contains(&params.redirect_uri) {
        return (StatusCode::BAD_REQUEST, "redirect_uri not registered for this client").into_response();
    }

    // From here, errors go back to the client via the redirect.
    if params.response_type != "code" {
        return authorize_error(&params.redirect_uri, &params.state, "unsupported_response_type");
    }
    if params.code_challenge.is_empty() || params.code_challenge_method != "S256" {
        return authorize_error(&params.redirect_uri, &params.state, "invalid_request");
    }

    // Identity: must be a logged-in pray.rs session. If not, bounce through login
    // and come back to this exact URL.
    let user_id = match session.get::<UserId>("user_id").await {
        Ok(Some(uid)) => uid,
        _ => {
            let return_to = format!("/authorize?{}", raw.unwrap_or_default());
            let login = format!("/login?return_to={}", urlencoding::encode(&return_to));
            return Redirect::to(&login).into_response();
        }
    };

    // Logged in → mint a one-time authorization code (v1 auto-approves consent).
    match oauth::create_auth_code(
        &state.db.pool,
        &params.client_id,
        user_id,
        &params.redirect_uri,
        &params.code_challenge,
        params.scope.as_deref(),
        params.resource.as_deref(),
        CODE_TTL_SECS,
    ).await {
        Ok(code) => {
            let mut q = format!("code={}", urlencoding::encode(&code));
            if let Some(s) = &params.state {
                q.push_str(&format!("&state={}", urlencoding::encode(s)));
            }
            Redirect::to(&with_query(&params.redirect_uri, &q)).into_response()
        }
        Err(_) => authorize_error(&params.redirect_uri, &params.state, "server_error"),
    }
}

// ── Token endpoint ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct TokenParams {
    pub grant_type: String,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub redirect_uri: Option<String>,
    #[serde(default)]
    pub code_verifier: Option<String>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub client_id: Option<String>,
}

fn token_error(error: &str) -> Response {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": error }))).into_response()
}

/// base64url(SHA-256(verifier)) — the S256 PKCE transform (RFC 7636).
fn pkce_s256(verifier: &str) -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use sha2::{Digest, Sha256};
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

fn token_response(t: oauth::IssuedTokens) -> Response {
    Json(json!({
        "access_token": t.access_token,
        "token_type": "Bearer",
        "expires_in": t.expires_in,
        "refresh_token": t.refresh_token,
        "scope": "mcp",
    })).into_response()
}

pub async fn token(
    Extension(state): Extension<AppState>,
    Form(params): Form<TokenParams>,
) -> Response {
    let pool = &state.db.pool;
    match params.grant_type.as_str() {
        "authorization_code" => {
            let (Some(code), Some(verifier)) = (params.code, params.code_verifier) else {
                return token_error("invalid_request");
            };

            let grant = match oauth::consume_auth_code(pool, &code).await {
                Ok(Some(g)) => g,
                Ok(None)    => return token_error("invalid_grant"),
                Err(_)      => return token_error("server_error"),
            };

            // PKCE: the verifier must hash to the stored challenge.
            if pkce_s256(&verifier) != grant.code_challenge {
                return token_error("invalid_grant");
            }
            // redirect_uri, if presented, must match the one bound to the code.
            if let Some(uri) = &params.redirect_uri {
                if uri != &grant.redirect_uri {
                    return token_error("invalid_grant");
                }
            }

            match oauth::issue_tokens(
                pool, &grant.client_id, grant.user_id, grant.scope.as_deref(), ACCESS_TTL_SECS,
            ).await {
                Ok(t)  => token_response(t),
                Err(_) => token_error("server_error"),
            }
        }
        "refresh_token" => {
            let Some(refresh) = params.refresh_token else {
                return token_error("invalid_request");
            };
            let ctx = match oauth::consume_refresh(pool, &refresh).await {
                Ok(Some(c)) => c,
                Ok(None)    => return token_error("invalid_grant"),
                Err(_)      => return token_error("server_error"),
            };
            match oauth::issue_tokens(
                pool, &ctx.client_id, ctx.user_id, ctx.scope.as_deref(), ACCESS_TTL_SECS,
            ).await {
                Ok(t)  => token_response(t),
                Err(_) => token_error("server_error"),
            }
        }
        _ => token_error("unsupported_grant_type"),
    }
}
