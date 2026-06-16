use axum::{
    extract::Extension,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use mcp_authorization::{AuthContext, Capability};
use serde_json::{json, Value};
use tower_sessions::Session;

use super::pagination::Pagination;

use crate::server::AppState;
use thanksgivings_core::UserId;

use super::{capabilities::*, tools};

// ── Auth resolution ───────────────────────────────────────────────────────────
//
// One endpoint, multiple credential layers. We try, in order:
//   1. Bearer token → static API key (header-capable clients: Claude Code/Desktop)
//   2. Bearer token → OAuth access token (claude.ai web, after the OAuth flow)
//   3. Session cookie → logged-in pray.rs browser tab
//   4. Anonymous → public read only
// The first layer that resolves a user wins.

fn anon() -> (Option<UserId>, AuthContext) {
    (None, AuthContext::new(vec![ReadPublic::NAME]))
}

async fn caps_for(user_id: UserId, state: &AppState) -> (Option<UserId>, AuthContext) {
    let has_groups = thanksgivings_db::repository::groups::list_for_user(
        &state.db.pool, user_id,
    ).await.map(|g| !g.is_empty()).unwrap_or(false);

    let mut caps = vec![ReadPublic::NAME, ReadOwn::NAME, WriteOwn::NAME, ManageGroups::NAME];
    if has_groups {
        caps.push(ReadGroup::NAME);
        caps.push(ShareToGroup::NAME);
    }
    (Some(user_id), AuthContext::new(caps))
}

async fn resolve_auth(headers: &HeaderMap, session: &Session, state: &AppState)
    -> (Option<UserId>, AuthContext)
{
    let bearer = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.trim().to_string());

    if let Some(token) = bearer {
        // Layer 1: static API key.
        if let Ok(Some(uid)) =
            thanksgivings_db::repository::api_keys::authenticate(&state.db.pool, &token).await
        {
            return caps_for(uid, state).await;
        }
        // Layer 2: OAuth access token.
        if let Ok(Some(uid)) =
            thanksgivings_db::repository::oauth::authenticate(&state.db.pool, &token).await
        {
            return caps_for(uid, state).await;
        }
    }

    // Layer 3: browser session cookie.
    if let Ok(Some(uid)) = session.get::<UserId>("user_id").await {
        return caps_for(uid, state).await;
    }

    // Layer 4: anonymous.
    anon()
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn mcp_ok(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn mcp_err(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn base_url() -> String {
    std::env::var("BASE_URL").unwrap_or_else(|_| "https://pray.rs".to_string())
}

/// A 401 with the RFC 9728 `WWW-Authenticate` challenge that points OAuth-capable
/// clients (claude.ai) at our protected-resource metadata, kicking off the flow.
/// Only capability-gated tools hit this — public read never does.
fn challenge(id: Value) -> Response {
    let metadata = format!("{}/.well-known/oauth-protected-resource", base_url());
    let www_auth = format!("Bearer resource_metadata=\"{metadata}\"");
    let body = mcp_err(id, -32001, "authentication required");

    let mut resp = (StatusCode::UNAUTHORIZED, Json(body)).into_response();
    if let Ok(val) = www_auth.parse() {
        resp.headers_mut().insert(header::WWW_AUTHENTICATE, val);
    }
    resp
}

/// Marker returned by gated tool branches when the caller isn't authorized,
/// so the dispatcher can answer with a scoped 401 instead of a 200 body.
struct NeedsAuth;

// ── Tool list ─────────────────────────────────────────────────────────────────

fn tools_list(auth: &AuthContext) -> Value {
    let mut tools: Vec<Value> = vec![];

    // ReadPublic — always visible
    tools.push(json!({
        "name": "list_public_prayers",
        "description": "List public prayers from the community.",
        "inputSchema": { "type": "object", "properties": Pagination::schema() }
    }));

    // ReadOwn — authenticated users
    if auth.require::<ReadOwn>().is_ok() {
        tools.push(json!({
            "name": "list_my_prayers",
            "description": "List all your own prayers at any visibility.",
            "inputSchema": { "type": "object", "properties": Pagination::schema() }
        }));
    }

    // WriteOwn — authenticated users
    if auth.require::<WriteOwn>().is_ok() {
        tools.push(json!({
            "name": "create_prayer",
            "description": "Write a new prayer entry.",
            "inputSchema": { "type": "object", "required": ["content"], "properties": {
                "content":    { "type": "string" },
                "visibility": { "type": "string", "enum": ["private", "public"], "default": "private" }
            }}
        }));
        tools.push(json!({
            "name": "edit_prayer",
            "description": "Replace the text of one of your own prayers.",
            "inputSchema": { "type": "object", "required": ["post_id", "content"], "properties": {
                "post_id": { "type": "string" },
                "content": { "type": "string" }
            }}
        }));
        tools.push(json!({
            "name": "delete_prayer",
            "description": "Permanently delete one of your own prayers.",
            "inputSchema": { "type": "object", "required": ["post_id"], "properties": {
                "post_id": { "type": "string" }
            }}
        }));
        tools.push(json!({
            "name": "set_visibility",
            "description": "Change visibility of one of your own prayers.",
            "inputSchema": { "type": "object", "required": ["post_id", "visibility"], "properties": {
                "post_id":    { "type": "string" },
                "visibility": { "type": "string", "description": "'private', 'public', or a group UUID." }
            }}
        }));
    }

    // ManageGroups — authenticated users
    if auth.require::<ManageGroups>().is_ok() {
        tools.push(json!({
            "name": "list_my_groups",
            "description": "List all prayer groups you belong to.",
            "inputSchema": { "type": "object", "properties": {} }
        }));
        tools.push(json!({
            "name": "create_group",
            "description": "Create a new prayer group.",
            "inputSchema": { "type": "object", "required": ["name"], "properties": {
                "name": { "type": "string" }
            }}
        }));
        tools.push(json!({
            "name": "invite_to_group",
            "description": "Send an email invitation to join one of your groups.",
            "inputSchema": { "type": "object", "required": ["group_id", "email"], "properties": {
                "group_id": { "type": "string" },
                "email":    { "type": "string" }
            }}
        }));
    }

    // ReadGroup — authenticated + group member
    if auth.require::<ReadGroup>().is_ok() {
        let mut props = Pagination::schema();
        props["group_id"] = json!({ "type": "string" });
        tools.push(json!({
            "name": "list_group_prayers",
            "description": "List prayers shared to a group you belong to.",
            "inputSchema": { "type": "object", "required": ["group_id"], "properties": props }
        }));
    }

    // ShareToGroup — authenticated + group member
    if auth.require::<ShareToGroup>().is_ok() {
        tools.push(json!({
            "name": "share_to_group",
            "description": "Share one of your prayers to a group you belong to.",
            "inputSchema": { "type": "object", "required": ["post_id", "group_id"], "properties": {
                "post_id":  { "type": "string" },
                "group_id": { "type": "string" }
            }}
        }));
    }

    json!({ "tools": tools })
}

// ── Axum handler ──────────────────────────────────────────────────────────────

pub async fn handle(
    Extension(state): Extension<AppState>,
    session: Session,
    headers: HeaderMap,
    Json(request): Json<Value>,
) -> Response {
    let id     = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request["method"].as_str().unwrap_or("");
    let params = request.get("params").cloned().unwrap_or(json!({}));

    match method {
        "initialize" => Json(mcp_ok(id, json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "prayers-mcp", "version": "0.1.0" }
        }))).into_response(),

        "tools/list" => {
            let (_, auth) = resolve_auth(&headers, &session, &state).await;
            Json(mcp_ok(id, tools_list(&auth))).into_response()
        }

        "tools/call" => {
            let (user_id, auth) = resolve_auth(&headers, &session, &state).await;
            let tool = params["name"].as_str().unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            let pool = &state.db.pool;

            // Ok(value) → 200 body; Err(NeedsAuth) → scoped 401 OAuth challenge.
            let result: Result<Value, NeedsAuth> = match tool {
                "list_public_prayers" => match auth.require::<ReadPublic>() {
                    Ok(proof) => Ok(tools::list_public_prayers(proof, pool, Pagination::from_args(&args)).await),
                    Err(_)    => Err(NeedsAuth),
                },
                "list_my_prayers" => match (auth.require::<ReadOwn>(), user_id) {
                    (Ok(proof), Some(uid)) => Ok(tools::list_my_prayers(proof, uid, pool, Pagination::from_args(&args)).await),
                    _ => Err(NeedsAuth),
                },
                "create_prayer" => match (auth.require::<WriteOwn>(), user_id) {
                    (Ok(proof), Some(uid)) => {
                        let content = args["content"].as_str().unwrap_or("").to_string();
                        let vis     = args["visibility"].as_str().unwrap_or("private").to_string();
                        if content.is_empty() {
                            Ok(json!({ "content": [{ "type": "text", "text": "'content' is required." }], "isError": true }))
                        } else {
                            Ok(tools::create_prayer(proof, uid, pool, content, vis).await)
                        }
                    }
                    _ => Err(NeedsAuth),
                },
                "set_visibility" => match (auth.require::<WriteOwn>(), user_id) {
                    (Ok(proof), Some(uid)) => {
                        let post_id = args["post_id"].as_str().unwrap_or("").to_string();
                        let vis     = args["visibility"].as_str().unwrap_or("private").to_string();
                        Ok(tools::set_visibility(proof, uid, pool, post_id, vis).await)
                    }
                    _ => Err(NeedsAuth),
                },
                "list_group_prayers" => match (auth.require::<ReadGroup>(), user_id) {
                    (Ok(proof), Some(uid)) => {
                        let group_id = args["group_id"].as_str().unwrap_or("").to_string();
                        Ok(tools::list_group_prayers(proof, uid, pool, group_id, Pagination::from_args(&args)).await)
                    }
                    _ => Err(NeedsAuth),
                },
                "share_to_group" => match (auth.require::<ShareToGroup>(), user_id) {
                    (Ok(proof), Some(uid)) => {
                        let post_id  = args["post_id"].as_str().unwrap_or("").to_string();
                        let group_id = args["group_id"].as_str().unwrap_or("").to_string();
                        Ok(tools::share_to_group(proof, uid, pool, post_id, group_id).await)
                    }
                    _ => Err(NeedsAuth),
                },
                "delete_prayer" => match (auth.require::<WriteOwn>(), user_id) {
                    (Ok(proof), Some(uid)) => {
                        let post_id = args["post_id"].as_str().unwrap_or("").to_string();
                        Ok(tools::delete_prayer(proof, uid, pool, post_id).await)
                    }
                    _ => Err(NeedsAuth),
                },
                "edit_prayer" => match (auth.require::<WriteOwn>(), user_id) {
                    (Ok(proof), Some(uid)) => {
                        let post_id = args["post_id"].as_str().unwrap_or("").to_string();
                        let content = args["content"].as_str().unwrap_or("").to_string();
                        Ok(tools::edit_prayer(proof, uid, pool, post_id, content).await)
                    }
                    _ => Err(NeedsAuth),
                },
                "list_my_groups" => match (auth.require::<ManageGroups>(), user_id) {
                    (Ok(proof), Some(uid)) => Ok(tools::list_my_groups(proof, uid, pool).await),
                    _ => Err(NeedsAuth),
                },
                "create_group" => match (auth.require::<ManageGroups>(), user_id) {
                    (Ok(proof), Some(uid)) => {
                        let name = args["name"].as_str().unwrap_or("").to_string();
                        Ok(tools::create_group(proof, uid, pool, name).await)
                    }
                    _ => Err(NeedsAuth),
                },
                "invite_to_group" => match (auth.require::<ManageGroups>(), user_id) {
                    (Ok(proof), Some(uid)) => {
                        let group_id = args["group_id"].as_str().unwrap_or("").to_string();
                        let email    = args["email"].as_str().unwrap_or("").to_string();
                        Ok(tools::invite_to_group(proof, uid, pool, group_id, email).await)
                    }
                    _ => Err(NeedsAuth),
                },
                other => Ok(json!({ "content": [{ "type": "text", "text": format!("Unknown tool: {other}") }], "isError": true })),
            };

            match result {
                Ok(value)      => Json(mcp_ok(id, value)).into_response(),
                Err(NeedsAuth) => challenge(id),
            }
        }

        "notifications/initialized" | "ping" => Json(mcp_ok(id, json!({}))).into_response(),

        _ => Json(mcp_err(id, -32601, "method not found")).into_response(),
    }
}
