use leptos::prelude::*;
use leptosbook::prelude::*;
use leptos_router::hooks::use_navigate;
use thanksgivings_core::{Post, VisibilityFilter};

use crate::components::{copyright::CopyrightNotice, post_page::PostPage, visibility_picker::VisibilityPicker, wordmark::Wordmark};

#[server]
pub async fn fetch_posts(filter: VisibilityFilter) -> Result<Vec<Post>, ServerFnError> {
    use crate::server::AppState;
    use thanksgivings_core::UserId;
    use tower_sessions::Session;

    let state = use_context::<AppState>()
        .ok_or_else(|| ServerFnError::new("missing app state"))?;
    let session = leptos_axum::extract::<Session>().await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let user_id: Option<UserId> = session.get("user_id").await.ok().flatten();

    match user_id {
        None      => Err(ServerFnError::new("not authenticated")),
        Some(uid) => {
            thanksgivings_db::repository::posts::list_for_viewer(
                &state.db.pool, uid, &filter,
            )
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))
        }
    }
}

#[component]
pub fn BookPage() -> impl IntoView {
    let (tab_index, set_tab_index) = signal(0usize);
    let (filter, set_filter)       = signal(VisibilityFilter::Mine);

    let posts = LocalResource::new(move || fetch_posts(filter.get()));

    let navigate = use_navigate();

    let nav_for_empty = navigate.clone();
    let empty_fb: leptos::prelude::ChildrenFn = std::sync::Arc::new(move || {
        let nav = nav_for_empty.clone();
        view! {
            <div class="empty-page">
                <p>"Begin your prayer book."</p>
                <button
                    class="compose-btn"
                    on:click=move |_| nav("/compose", Default::default())
                >
                    "+ Write your first prayer"
                </button>
            </div>
        }.into_any()
    });

    // Redirect to /login when the server says not authenticated.
    let nav_auth = navigate.clone();
    Effect::new(move |_| {
        if let Some(Err(e)) = posts.get() {
            if e.to_string().contains("not authenticated") {
                nav_auth("/login", Default::default());
            }
        }
    });

    let items: Signal<Vec<Post>> = Signal::derive(move || {
        posts.get()
            .and_then(|r| r.ok())
            .unwrap_or_default()
    });

    let tabs = vec!["Mine", "Public"];

    view! {
        <div class="book">
            <header class="book-header">
                <Wordmark/>
                <VisibilityPicker
                    value=filter
                    on_change=move |v| set_filter.set(v)
                />
            </header>

            // ── leptoskit takes over everything below ──────────────────
            <Folio
                items=items
                render=|post: Post| view! { <PostPage post=post/> }
                threshold=60.0
                empty_fallback=empty_fb
            >
                <footer class="book-footer">
                    <FolioNav/>
                    <div class="footer-actions">
                        {let nav_g = navigate.clone(); view! {
                            <button
                                class="groups-btn"
                                on:click=move |_| nav_g("/groups", Default::default())
                                title="Prayer groups"
                            >
                                "👥"
                            </button>
                        }}
                        {let nav_s = navigate.clone(); view! {
                            <button
                                class="groups-btn"
                                on:click=move |_| nav_s("/settings", Default::default())
                                title="Connect your AI"
                            >
                                "⚙"
                            </button>
                        }}
                        <button
                            class="compose-btn"
                            on:click=move |_| navigate("/compose", Default::default())
                        >
                            "+ New"
                        </button>
                    </div>
                </footer>
            </Folio>
            <CopyrightNotice/>
            <InstallPrompt/>
        </div>
    }
}
