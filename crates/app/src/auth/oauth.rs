use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_query_map};

// ─── Login Page ───────────────────────────────────────────────────────────────

#[server]
pub async fn dev_login(return_to: Option<String>) -> Result<String, ServerFnError> {
    use tower_sessions::Session;

    if std::env::var("DEV_AUTH_BYPASS").as_deref() != Ok("1") {
        return Err(ServerFnError::new("not available"));
    }
    let session = leptos_axum::extract::<Session>().await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let app_state = use_context::<crate::server::AppState>()
        .ok_or_else(|| ServerFnError::new("missing app state"))?;

    let user = thanksgivings_db::repository::users::upsert_oauth_user(
        &app_state.db.pool,
        &thanksgivings_core::OAuthProvider::Google,
        "dev-user",
        "dev@localhost",
        "Dev User",
        None,
    ).await.map_err(|e| ServerFnError::new(e.to_string()))?;

    session.insert("user_id", user.id).await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(return_to.filter(|s| !s.is_empty()).unwrap_or_else(|| "/".to_string()))
}

#[server]
pub async fn login_url(provider: String, return_to: Option<String>) -> Result<String, ServerFnError> {
    use oauth2::{CsrfToken, PkceCodeChallenge, Scope};
    use tower_sessions::Session;

    let session = leptos_axum::extract::<Session>().await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let csrf_token = CsrfToken::new_random();

    session.insert("pkce_verifier", pkce_verifier.secret().clone()).await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    session.insert("csrf_token", csrf_token.secret().clone()).await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    session.insert("oauth_provider", provider.clone()).await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    // Where to land after login completes (e.g. an in-flight /authorize request).
    session.insert("post_login_redirect", return_to.unwrap_or_default()).await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let (url, _) = match provider.as_str() {
        "google" => {
            let client = google_client()?;
            client
                .authorize_url(|| csrf_token)
                .add_scope(Scope::new("openid".into()))
                .add_scope(Scope::new("email".into()))
                .add_scope(Scope::new("profile".into()))
                .set_pkce_challenge(pkce_challenge)
                .url()
        }
        "facebook" => {
            let client = facebook_client()?;
            client
                .authorize_url(|| csrf_token)
                .add_scope(Scope::new("email".into()))
                .add_scope(Scope::new("public_profile".into()))
                .set_pkce_challenge(pkce_challenge)
                .url()
        }
        _ => return Err(ServerFnError::new("unknown provider")),
    };

    Ok(url.to_string())
}

#[component]
pub fn LoginPage() -> impl IntoView {
    let navigate = use_navigate();
    let nav_dev  = navigate.clone();
    let query    = use_query_map();

    let start_oauth = Action::new(move |provider: &String| {
        let provider  = provider.clone();
        let nav       = navigate.clone();
        let return_to = query.read().get("return_to");
        async move {
            match login_url(provider, return_to).await {
                Ok(url) => {
                    let _ = web_sys::window()
                        .and_then(|w| w.location().set_href(&url).ok());
                }
                Err(_) => { nav("/login", Default::default()); }
            }
        }
    });

    let dev_action = Action::new(move |_: &()| {
        let nav       = nav_dev.clone();
        let return_to = query.read().get("return_to");
        async move {
            if let Ok(redirect) = dev_login(return_to).await {
                // /authorize is a server route — needs a full page load, not SPA nav.
                if redirect.starts_with("/authorize") {
                    let _ = web_sys::window()
                        .and_then(|w| w.location().set_href(&redirect).ok());
                } else {
                    nav(&redirect, Default::default());
                }
            }
        }
    });

    view! {
        <div class="book">
            <div class="page-content login-page">
                <div class="login-wordmark"><crate::components::wordmark::Wordmark/></div>
                <p class="post-meta">"A prayer book for the digital age"</p>
                <div class="login-buttons">
                    <button class="compose-btn" on:click=move |_| { start_oauth.dispatch("google".to_string()); }>
                        "Continue with Google"
                    </button>
                    <button class="compose-btn" on:click=move |_| { start_oauth.dispatch("facebook".to_string()); }>
                        "Continue with Facebook"
                    </button>
                    {
                        #[cfg(debug_assertions)]
                        view! {
                            <button class="compose-btn" style="opacity:0.5;font-size:0.8rem"
                                on:click=move |_| { dev_action.dispatch(()); }>
                                "⚡ Dev login"
                            </button>
                        }
                        #[cfg(not(debug_assertions))]
                        view! { <></> }
                    }
                </div>
            </div>
            <crate::components::copyright::CopyrightNotice/>
        </div>
    }
}

// ─── Callback Page ────────────────────────────────────────────────────────────

#[server]
pub async fn handle_callback(
    code: String,
    state_param: String,
) -> Result<String, ServerFnError> {
    use oauth2::{AuthorizationCode, PkceCodeVerifier, TokenResponse};
    use tower_sessions::Session;
    use serde::Deserialize;

    let session = leptos_axum::extract::<Session>().await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let app_state = use_context::<crate::server::AppState>()
        .ok_or_else(|| ServerFnError::new("missing app state"))?;

    let stored_csrf: Option<String> = session.get("csrf_token").await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    if stored_csrf.as_deref() != Some(&state_param) {
        return Err(ServerFnError::new("CSRF mismatch"));
    }

    let verifier_secret: String = session.get("pkce_verifier").await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("missing verifier"))?;

    let provider: String = session.get("oauth_provider").await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .unwrap_or_else(|| "google".to_string());

    let http = reqwest::Client::new();

    #[derive(Deserialize)]
    struct GoogleProfile { sub: String, email: String, name: String, picture: Option<String> }
    #[derive(Deserialize)]
    struct FbProfile    { id: String,  email: String, name: String }

    let (prov_enum, pid, email, name, avatar) = match provider.as_str() {
        "google" => {
            let client = google_client()?;
            let token = client
                .exchange_code(AuthorizationCode::new(code))
                .set_pkce_verifier(PkceCodeVerifier::new(verifier_secret))
                .request_async(oauth2::reqwest::async_http_client)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;

            let profile: GoogleProfile = http
                .get("https://www.googleapis.com/oauth2/v3/userinfo")
                .bearer_auth(token.access_token().secret())
                .send().await
                .map_err(|e| ServerFnError::new(e.to_string()))?
                .json().await
                .map_err(|e| ServerFnError::new(e.to_string()))?;

            (thanksgivings_core::OAuthProvider::Google, profile.sub, profile.email, profile.name, profile.picture)
        }
        "facebook" => {
            let client = facebook_client()?;
            let token = client
                .exchange_code(AuthorizationCode::new(code))
                .set_pkce_verifier(PkceCodeVerifier::new(verifier_secret))
                .request_async(oauth2::reqwest::async_http_client)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;

            let profile: FbProfile = http
                .get("https://graph.facebook.com/me?fields=id,name,email")
                .bearer_auth(token.access_token().secret())
                .send().await
                .map_err(|e| ServerFnError::new(e.to_string()))?
                .json().await
                .map_err(|e| ServerFnError::new(e.to_string()))?;

            (thanksgivings_core::OAuthProvider::Facebook, profile.id, profile.email, profile.name, None)
        }
        _ => return Err(ServerFnError::new("unknown provider")),
    };

    let user = thanksgivings_db::repository::users::upsert_oauth_user(
        &app_state.db.pool,
        &prov_enum,
        &pid,
        &email,
        &name,
        avatar.as_deref(),
    )
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    session.insert("user_id", user.id).await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let redirect: Option<String> = session.get("post_login_redirect").await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let _ = session.remove::<String>("post_login_redirect").await;

    Ok(redirect.filter(|s| !s.is_empty()).unwrap_or_else(|| "/".to_string()))
}

#[component]
pub fn CallbackPage() -> impl IntoView {
    let navigate = use_navigate();
    let query    = use_query_map();

    let callback = Action::new(move |(code, state): &(String, String)| {
        let code  = code.clone();
        let state = state.clone();
        let nav   = navigate.clone();
        async move {
            match handle_callback(code, state).await {
                Ok(redirect) => {
                    // /authorize is a server route — needs a full page load, not SPA nav.
                    if redirect.starts_with("/authorize") {
                        let _ = web_sys::window()
                            .and_then(|w| w.location().set_href(&redirect).ok());
                    } else {
                        nav(&redirect, Default::default());
                    }
                }
                Err(_) => { nav("/login", Default::default()); }
            }
        }
    });

    // Extract code + state from router query map
    Effect::new(move |_| {
        let q     = query.read();
        let code  = q.get("code").clone().unwrap_or_default();
        let state = q.get("state").clone().unwrap_or_default();
        if !code.is_empty() {
            callback.dispatch((code, state));
        }
    });

    view! {
        <div class="book">
            <div class="page-content">
                <p class="loading">"Signing you in..."</p>
            </div>
        </div>
    }
}

// ─── OAuth clients (SSR only) ─────────────────────────────────────────────────

#[cfg(feature = "ssr")]
pub struct OAuthConfig {
    pub google_client_id:     String,
    pub google_client_secret: String,
    pub facebook_client_id:     String,
    pub facebook_client_secret: String,
    pub base_url: String,
}

#[cfg(feature = "ssr")]
impl OAuthConfig {
    pub fn from_env() -> Self {
        Self {
            google_client_id:       std::env::var("GOOGLE_CLIENT_ID").expect("GOOGLE_CLIENT_ID"),
            google_client_secret:   std::env::var("GOOGLE_CLIENT_SECRET").expect("GOOGLE_CLIENT_SECRET"),
            facebook_client_id:     std::env::var("FACEBOOK_CLIENT_ID").expect("FACEBOOK_CLIENT_ID"),
            facebook_client_secret: std::env::var("FACEBOOK_CLIENT_SECRET").expect("FACEBOOK_CLIENT_SECRET"),
            base_url: std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string()),
        }
    }
}

#[cfg(feature = "ssr")]
pub fn google_client() -> Result<oauth2::basic::BasicClient, ServerFnError> {
    use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl, basic::BasicClient};
    let cfg = OAuthConfig::from_env();
    Ok(BasicClient::new(
        ClientId::new(cfg.google_client_id),
        Some(ClientSecret::new(cfg.google_client_secret)),
        AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".into())
            .map_err(|e| ServerFnError::new(e.to_string()))?,
        Some(TokenUrl::new("https://oauth2.googleapis.com/token".into())
            .map_err(|e| ServerFnError::new(e.to_string()))?),
    )
    .set_redirect_uri(
        RedirectUrl::new(format!("{}/auth/callback/google", cfg.base_url))
            .map_err(|e| ServerFnError::new(e.to_string()))?,
    ))
}

#[cfg(feature = "ssr")]
pub fn facebook_client() -> Result<oauth2::basic::BasicClient, ServerFnError> {
    use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl, basic::BasicClient};
    let cfg = OAuthConfig::from_env();
    Ok(BasicClient::new(
        ClientId::new(cfg.facebook_client_id),
        Some(ClientSecret::new(cfg.facebook_client_secret)),
        AuthUrl::new("https://www.facebook.com/v19.0/dialog/oauth".into())
            .map_err(|e| ServerFnError::new(e.to_string()))?,
        Some(TokenUrl::new("https://graph.facebook.com/v19.0/oauth/access_token".into())
            .map_err(|e| ServerFnError::new(e.to_string()))?),
    )
    .set_redirect_uri(
        RedirectUrl::new(format!("{}/auth/callback/facebook", cfg.base_url))
            .map_err(|e| ServerFnError::new(e.to_string()))?,
    ))
}
