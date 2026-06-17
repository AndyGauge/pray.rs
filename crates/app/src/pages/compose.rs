use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use thanksgivings_core::{Group, NewPost, PostKind, Visibility};

use crate::pages::groups::fetch_my_groups;

#[server]
pub async fn create_post(new_post: NewPost) -> Result<(), ServerFnError> {
    use crate::server::AppState;
    use thanksgivings_core::UserId;
    use tower_sessions::Session;

    let state = use_context::<AppState>()
        .ok_or_else(|| ServerFnError::new("missing app state"))?;
    let session = leptos_axum::extract::<Session>().await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let user_id: Option<UserId> = session.get("user_id").await.ok().flatten();
    let uid = user_id.ok_or_else(|| ServerFnError::new("not authenticated"))?;

    // Never trust the client: reject empty content (the UI checks too).
    if new_post.content.trim().is_empty() {
        return Err(ServerFnError::new("Prayer cannot be empty."));
    }

    // If the post is addressed to a group, the author must belong to it —
    // otherwise a crafted request could inject a post into a group they are
    // not a member of. Mirrors the membership check on the share path.
    if let Visibility::Group(gid) = new_post.visibility {
        let groups = thanksgivings_db::repository::groups::list_for_user(&state.db.pool, uid)
            .await.map_err(|e| ServerFnError::new(e.to_string()))?;
        if !groups.iter().any(|g| g.id == gid) {
            return Err(ServerFnError::new("You are not a member of that group."));
        }
    }

    thanksgivings_db::repository::posts::create(&state.db.pool, uid, new_post)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}

#[component]
pub fn ComposePage() -> impl IntoView {
    let (kind, set_kind)       = signal(PostKind::Prayer);
    let (content, set_content) = signal(String::new());
    let (vis, set_vis)         = signal(Visibility::Private);
    let (error, set_error)     = signal(Option::<String>::None);

    let groups = LocalResource::new(fetch_my_groups);
    let groups_sig: Signal<Vec<Group>> = Signal::derive(move || {
        groups.get().and_then(|r| r.ok()).unwrap_or_default()
    });

    let navigate = use_navigate();
    let nav2     = navigate.clone();

    let submit = Action::new(move |post: &NewPost| {
        let post = post.clone();
        let nav  = navigate.clone();
        async move {
            match create_post(post).await {
                Ok(_)  => { nav("/", Default::default()); }
                Err(e) => { set_error.set(Some(e.to_string())); }
            }
        }
    });

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let text = content.get_untracked();
        if text.trim().is_empty() { return; }
        submit.dispatch(NewPost {
            kind:       kind.get_untracked(),
            content:    text,
            visibility: vis.get_untracked(),
            prayer_id:  None,
        });
    };

    view! {
        <div class="book">
            <header class="book-header">
                <button class="nav-btn" on:click=move |_| { nav2("/", Default::default()); }>"← Back"</button>
                <span class="book-title">"New Entry"</span>
            </header>

            <div class="page-content">
                <form class="compose-form" on:submit=on_submit>

                    // Kind picker
                    <div class="form-field">
                        <label>"Entry type"</label>
                        <div class="kind-picker">
                            <button
                                type="button"
                                class=move || if kind.get() == PostKind::Prayer { "kind-btn active" } else { "kind-btn" }
                                on:click=move |_| set_kind.set(PostKind::Prayer)
                            >"Prayer"</button>
                            <button
                                type="button"
                                class=move || if kind.get() == PostKind::Praise { "kind-btn active" } else { "kind-btn" }
                                on:click=move |_| set_kind.set(PostKind::Praise)
                            >"Praise"</button>
                        </div>
                    </div>

                    // Content
                    <div class="form-field">
                        <label>"Your words"</label>
                        <textarea
                            rows="8"
                            placeholder="Write your prayer or praise..."
                            on:input=move |ev| set_content.set(event_target_value(&ev))
                            prop:value=content
                        />
                    </div>

                    // Visibility
                    <VisibilityField vis=vis set_vis=set_vis groups=groups_sig/>

                    // Error
                    {move || error.get().map(|e| view! { <p class="error-msg">{e}</p> })}

                    <div class="form-actions">
                        <button type="submit" class="compose-btn" disabled=move || submit.pending().get()>
                            {move || if submit.pending().get() { "Saving..." } else { "Save" }}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    }
}

/// The "Who can see this" control: Private / Public / one button per group,
/// collapsing to a dropdown on mobile when the user has groups.
///
/// Extracted into its own component (returning `AnyView` via `.into_any()`) so
/// its deep view type doesn't inflate `ComposePage`'s — see CLAUDE.md.
#[component]
fn VisibilityField(
    vis: ReadSignal<Visibility>,
    set_vis: WriteSignal<Visibility>,
    groups: Signal<Vec<Group>>,
) -> impl IntoView {
    view! {
        <div class="form-field">
            <label>"Who can see this"</label>
            <div class=move || if groups.get().is_empty() { "vis-control" } else { "vis-control has-groups" }>
                // Button row (desktop, or always when there are no groups)
                <div class="kind-picker vis-buttons">
                    <button type="button"
                        class=move || if vis.get() == Visibility::Private { "kind-btn active" } else { "kind-btn" }
                        on:click=move |_| set_vis.set(Visibility::Private)
                    >"Private"</button>
                    <button type="button"
                        class=move || if vis.get() == Visibility::Public  { "kind-btn active" } else { "kind-btn" }
                        on:click=move |_| set_vis.set(Visibility::Public)
                    >"Public"</button>
                    {move || groups.get().into_iter().map(|g| {
                        let gid = g.id;
                        view! {
                            <button type="button"
                                class=move || if vis.get() == Visibility::Group(gid) { "kind-btn active" } else { "kind-btn" }
                                on:click=move |_| set_vis.set(Visibility::Group(gid))
                            >{g.name}</button>
                        }
                    }).collect_view()}
                </div>

                // Dropdown (mobile, only when there are groups)
                {move || (!groups.get().is_empty()).then(|| {
                    let gs = groups.get();
                    view! {
                        <select
                            class="vis-select"
                            prop:value=move || match vis.get() {
                                Visibility::Private   => "private".to_string(),
                                Visibility::Public    => "public".to_string(),
                                Visibility::Group(g)  => g.to_string(),
                            }
                            on:change=move |e| {
                                let v = event_target_value(&e);
                                let chosen = match v.as_str() {
                                    "public"  => Visibility::Public,
                                    "private" => Visibility::Private,
                                    other     => other.parse()
                                        .map(Visibility::Group)
                                        .unwrap_or(Visibility::Private),
                                };
                                set_vis.set(chosen);
                            }
                        >
                            <option value="private">"Private"</option>
                            <option value="public">"Public"</option>
                            {gs.into_iter().map(|g| {
                                let id = g.id.to_string();
                                view! { <option value=id>{g.name}</option> }
                            }).collect_view()}
                        </select>
                    }
                })}
            </div>
        </div>
    }
    .into_any()
}
