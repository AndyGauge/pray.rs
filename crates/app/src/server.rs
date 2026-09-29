use std::sync::Arc;
use thanksgivings_db::Db;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Db>,
}

// ── Request context ───────────────────────────────────────────────────────────

use axum::{extract::FromRequestParts, http::request::Parts};
use thanksgivings_core::UserId;
use tower_sessions::Session;

/// The signed-in user for this request, with the app state and session.
///
/// An Axum extractor, so it's injected the same way in server functions
/// (`let ctx: Authed = leptos_axum::extract().await?;`) and in plain Axum
/// handlers (as a handler argument). Extraction fails with
/// "not authenticated" when nobody is signed in, and the book redirects to
/// `/login` on exactly that message.
pub struct Authed {
    pub state:   AppState,
    pub user_id: UserId,
    pub session: Session,
}

impl Authed {
    pub fn pool(&self) -> &sqlx::SqlitePool {
        &self.state.db.pool
    }
}

/// Why `Authed` couldn't be extracted. `Debug` prints just the message:
/// `leptos_axum::extract` turns a rejection into `ServerError("{rejection:?}")`,
/// so this is the text the client sees.
pub struct AuthRejection(&'static str);

impl std::fmt::Debug for AuthRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl axum::response::IntoResponse for AuthRejection {
    fn into_response(self) -> axum::response::Response {
        (axum::http::StatusCode::UNAUTHORIZED, self.0).into_response()
    }
}

impl<S: Send + Sync> FromRequestParts<S> for Authed {
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app = parts.extensions.get::<AppState>().cloned()
            .ok_or(AuthRejection("missing app state"))?;
        let session = Session::from_request_parts(parts, state).await
            .map_err(|_| AuthRejection("missing session"))?;
        let user_id = session.get::<UserId>("user_id").await.ok().flatten()
            .ok_or(AuthRejection("not authenticated"))?;
        Ok(Authed { state: app, user_id, session })
    }
}

/// Extract [`Authed`] inside a server function, keeping the rejection message
/// ("not authenticated") exactly as the client expects it. This is what
/// `#[authed]` expands to.
pub async fn extract_authed() -> Result<Authed, leptos::prelude::ServerFnError> {
    use leptos::server_fn::error::ServerFnErrorErr;
    leptos_axum::extract::<Authed>().await.map_err(|e| match e {
        ServerFnErrorErr::ServerError(msg) => leptos::prelude::ServerFnError::new(msg),
        other => leptos::prelude::ServerFnError::new(other.to_string()),
    })
}
