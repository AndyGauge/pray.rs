use leptos::prelude::*;
use leptosbook::prelude::*;
use leptos_router::hooks::use_navigate;
use thanksgivings_core::{Group, PostState, PrayerCount, ViewedPost, VisibilityFilter};

use crate::components::{copyright::CopyrightNotice, post_page::PostPage, visibility_picker::VisibilityPicker, wordmark::Wordmark};
use crate::pages::groups::fetch_my_groups;

#[server]
pub async fn fetch_posts(filter: VisibilityFilter) -> Result<Vec<ViewedPost>, ServerFnError> {
    use crate::server::AppState;
    use thanksgivings_core::{UserId, Viewer};
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
            .map(|posts| posts.into_iter().map(|post| {
                let viewer = if post.author_id == uid { Viewer::Author } else { Viewer::Other };
                ViewedPost { post, viewer }
            }).collect())
            .map_err(|e| ServerFnError::new(e.to_string()))
        }
    }
}

/// "I'm praying for this now" (+1) on any prayer the caller can see. Every
/// call counts. Returns the new tally. Mirrors the MCP `pray_for` tool.
#[server]
pub async fn pray_for(post_id: String) -> Result<PrayerCount, ServerFnError> {
    use crate::server::AppState;
    use thanksgivings_core::UserId;
    use tower_sessions::Session;

    let state = use_context::<AppState>()
        .ok_or_else(|| ServerFnError::new("missing app state"))?;
    let session = leptos_axum::extract::<Session>().await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let uid: UserId = session.get("user_id").await.ok().flatten()
        .ok_or_else(|| ServerFnError::new("not authenticated"))?;

    // pray() checks the caller can see the post and that its state allows it.
    thanksgivings_db::repository::posts::pray(&state.db.pool, &post_id, uid)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("You can't pray for this entry."))
}

/// Share one of the caller's own prayers to a group they belong to, making it
/// visible to other group members. Mirrors the MCP `share_to_group` tool.
#[server]
pub async fn share_prayer_to_group(post_id: String, group_id: String) -> Result<(), ServerFnError> {
    use crate::server::AppState;
    use thanksgivings_core::{GroupId, UserId};
    use tower_sessions::Session;

    let state = use_context::<AppState>()
        .ok_or_else(|| ServerFnError::new("missing app state"))?;
    let session = leptos_axum::extract::<Session>().await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let uid: UserId = session.get("user_id").await.ok().flatten()
        .ok_or_else(|| ServerFnError::new("not authenticated"))?;

    let gid: GroupId = group_id.parse()
        .map_err(|_| ServerFnError::new("invalid group"))?;

    // Only members of the target group may share into it.
    let groups = thanksgivings_db::repository::groups::list_for_user(&state.db.pool, uid)
        .await.map_err(|e| ServerFnError::new(e.to_string()))?;
    if !groups.iter().any(|g| g.id == gid) {
        return Err(ServerFnError::new("You are not a member of that group."));
    }

    // set_visibility enforces author ownership at the DB level.
    let shared = thanksgivings_db::repository::posts::set_visibility(
        &state.db.pool, &post_id, uid, &group_id,
    ).await.map_err(|e| ServerFnError::new(e.to_string()))?;

    if shared { Ok(()) } else { Err(ServerFnError::new("Prayer not found or not yours.")) }
}

/// Move one of the caller's own entries along its lifecycle
/// (prayer → thanksgiving, either → released), recording an optional note
/// about the move. Mirrors the MCP `give_thanks` / `release_prayer` tools.
#[server]
pub async fn set_post_state(
    post_id: String,
    state: PostState,
    note: Option<String>,
) -> Result<(), ServerFnError> {
    use crate::server::AppState;
    use thanksgivings_core::UserId;
    use tower_sessions::Session;

    let state_ctx = use_context::<AppState>()
        .ok_or_else(|| ServerFnError::new("missing app state"))?;
    let session = leptos_axum::extract::<Session>().await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let uid: UserId = session.get("user_id").await.ok().flatten()
        .ok_or_else(|| ServerFnError::new("not authenticated"))?;

    // transition enforces both ownership and the allowed-from states.
    let moved = thanksgivings_db::repository::posts::transition(
        &state_ctx.db.pool, &post_id, uid, state, note.as_deref(),
    ).await.map_err(|e| ServerFnError::new(e.to_string()))?;

    if moved { Ok(()) } else { Err(ServerFnError::new("That change isn't possible for this entry.")) }
}

#[component]
pub fn BookPage() -> impl IntoView {
    let (tab_index, set_tab_index) = signal(0usize);
    let (filter, set_filter)       = signal(VisibilityFilter::Mine);

    let posts  = LocalResource::new(move || fetch_posts(filter.get()));
    let groups = LocalResource::new(fetch_my_groups);

    // The viewer's groups, available to each prayer's share control.
    let groups_sig: Signal<Vec<Group>> = Signal::derive(move || {
        groups.get().and_then(|r| r.ok()).unwrap_or_default()
    });
    // Prayers are only the viewer's own on the "Mine" tab, so only offer
    // sharing there.
    let can_share = Signal::derive(move || filter.get() == VisibilityFilter::Mine);

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

    let items: Signal<Vec<ViewedPost>> = Signal::derive(move || {
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
                    groups=groups_sig
                    on_change=move |v| set_filter.set(v)
                />
            </header>

            // ── leptoskit takes over everything below ──────────────────
            <Folio
                items=items
                render=move |v: ViewedPost| view! {
                    <PostPage
                        post=v.post
                        viewer=v.viewer
                        groups=groups_sig
                        can_share=can_share
                        on_state_change=Callback::new(move |_| posts.refetch())
                    />
                }
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
