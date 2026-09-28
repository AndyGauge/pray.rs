#[cfg(feature = "ssr")]
fn qr_svg(url: &str) -> String {
    use qrcode::{EcLevel, QrCode};
    let code = match QrCode::with_error_correction_level(url, EcLevel::M) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };
    let width = code.width();
    let colors = code.into_colors();
    let cell: u32 = 10;
    let margin: u32 = 4;
    let total: u32 = (width as u32 + 2 * margin) * cell;
    let mut rects = String::new();
    for (i, color) in colors.iter().enumerate() {
        if *color == qrcode::Color::Dark {
            let row = (i / width) as u32;
            let col = (i % width) as u32;
            let x = (col + margin) * cell;
            let y = (row + margin) * cell;
            rects.push_str(&format!(
                r#"<rect x="{x}" y="{y}" width="{cell}" height="{cell}"/>"#,
            ));
        }
    }
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {t} {t}" width="{t}" height="{t}"><rect width="100%" height="100%" fill="#fdf8f0"/><g fill="#1a0f08">{rects}</g></svg>"##,
        t = total,
    )
}

/// Sign out and wipe everything the browser holds for pray.rs: its HTTP cache
/// (including the WASM/JS bundle), cookies and storage, via `Clear-Site-Data`.
///
/// A plain server route, not a Leptos page, so it works from *any* client
/// version, including a stale cached bundle that predates the Settings button.
/// Opening https://pray.rs/logout in the phone's browser also resets an
/// installed home-screen app, which shares the browser's storage.
#[cfg(feature = "ssr")]
async fn logout_handler(session: tower_sessions::Session) -> impl axum::response::IntoResponse {
    use axum::http::header;

    let _ = session.flush().await;
    (
        [
            (header::HeaderName::from_static("clear-site-data"), r#""cache", "cookies", "storage""#),
            (header::CACHE_CONTROL, "no-store"),
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
        ],
        LOGOUT_PAGE,
    )
}

/// Shown while the browser clears its data, then sends the user to sign in.
#[cfg(feature = "ssr")]
const LOGOUT_PAGE: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="theme-color" content="#1a0f08">
<meta http-equiv="refresh" content="2; url=/login">
<title>Signed out · prayers</title>
<style>
  body { margin: 0; min-height: 100vh; display: grid; place-items: center;
         background: #fdf8f0; color: #2c1810; text-align: center;
         font-family: 'Georgia', 'Palatino', 'Times New Roman', serif; }
  a { color: #8b4513; }
  p { margin: 0.5rem 1.5rem; }
</style>
</head>
<body>
  <main>
    <p>You're signed out, and this device's copy of pray.rs has been cleared.</p>
    <p><a href="/login">Sign in again</a></p>
  </main>
</body>
</html>
"##;

#[cfg(feature = "ssr")]
async fn group_qr_svg_handler(
    axum::extract::Extension(state): axum::extract::Extension<app::server::AppState>,
    axum::extract::Path(group_id_str): axum::extract::Path<String>,
) -> impl axum::response::IntoResponse {
    use axum::http::{header, StatusCode};
    use thanksgivings_core::GroupId;

    let Ok(gid) = group_id_str.parse::<GroupId>() else {
        return (StatusCode::BAD_REQUEST, [(header::CONTENT_TYPE, "text/plain")], "bad group id".to_string());
    };

    let token = match thanksgivings_db::repository::group_join_tokens::get_for_group(&state.db.pool, gid).await {
        Ok(Some(t)) => t,
        _ => return (StatusCode::NOT_FOUND, [(header::CONTENT_TYPE, "text/plain")], "no join link".to_string()),
    };

    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "https://pray.rs".to_string());
    let join_url = format!("{}/join/{}", base_url, token);
    let svg = qr_svg(&join_url);

    (StatusCode::OK, [(header::CONTENT_TYPE, "image/svg+xml")], svg)
}

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::Router;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use std::sync::Arc;
    use tower_http::compression::CompressionLayer;
    use tower_sessions::{cookie::SameSite, SessionManagerLayer};
    use tower_sessions_sqlx_store::SqliteStore;

    use app::{app::App, server::AppState};

    dotenvy::from_path(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.env")
    ).ok();
    dotenvy::dotenv().ok();

    // Build-time default ensures the DB file lands at the workspace root
    // regardless of the CWD when the binary is spawned by cargo-leptos.
    let db_default = concat!(
        "sqlite://",
        env!("CARGO_MANIFEST_DIR"),
        "/../../thanksgivings.db"
    );

    let conf          = get_configuration(None).unwrap();
    let addr          = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options.clone();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| db_default.to_string());

    let db = Arc::new(
        thanksgivings_db::Db::open(&database_url)
            .await
            .expect("failed to open database"),
    );

    let session_store = SqliteStore::new(db.pool.clone());
    session_store.migrate().await.expect("session migration failed");

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_same_site(SameSite::Lax);

    let app_state = AppState { db };
    let routes     = generate_route_list(App);

    use app::oauth_server;
    let router = Router::new()
        .route("/mcp", axum::routing::post(app::mcp::handler::handle))
        // OAuth 2.1 authorization server for the MCP endpoint.
        .route("/.well-known/oauth-protected-resource",
            axum::routing::get(oauth_server::protected_resource_metadata))
        .route("/.well-known/oauth-authorization-server",
            axum::routing::get(oauth_server::authorization_server_metadata))
        .route("/register",  axum::routing::post(oauth_server::register))
        .route("/authorize", axum::routing::get(oauth_server::authorize))
        .route("/token",     axum::routing::post(oauth_server::token))
        .route("/groups/{id}/qr.svg", axum::routing::get(group_qr_svg_handler))
        .route("/logout", axum::routing::get(logout_handler))
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            {
                let state = app_state.clone();
                move || provide_context(state.clone())
            },
            {
                let opts = leptos_options.clone();
                move || shell(opts.clone())
            },
        )
        .fallback(leptos_axum::file_and_error_handler(shell))
        .layer(axum::Extension(app_state))
        .layer(session_layer)
        .layer(CompressionLayer::new())
        .with_state(leptos_options);

    println!("Listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, router).await.unwrap();
}

#[cfg(feature = "ssr")]
fn shell(options: leptos::prelude::LeptosOptions) -> impl leptos::prelude::IntoView {
    use app::app::App;
    use leptos::prelude::*;
    use leptos_meta::MetaTags;

    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <title>"prayers"</title>
                <link rel="stylesheet" href="/pkg/thanksgivings.css"/>
                <link rel="icon" type_="image/png" href="/logo.png"/>
                <link rel="apple-touch-icon" href="/logo.png"/>
                <link rel="manifest" href="/manifest.json"/>
                <meta name="theme-color" content="#1a0f08"/>
                <meta name="apple-mobile-web-app-capable" content="yes"/>
                <meta name="apple-mobile-web-app-status-bar-style" content="black-translucent"/>
                <meta name="apple-mobile-web-app-title" content="prayers"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options=options.clone()/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[cfg(not(feature = "ssr"))]
pub fn main() {}
