use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use thanksgivings_core::{NewPost, PostKind, Visibility};

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
                    <div class="form-field">
                        <label>"Who can see this"</label>
                        <div class="kind-picker">
                            <button type="button"
                                class=move || if vis.get() == Visibility::Private { "kind-btn active" } else { "kind-btn" }
                                on:click=move |_| set_vis.set(Visibility::Private)
                            >"Private"</button>
                            <button type="button"
                                class=move || if vis.get() == Visibility::Public  { "kind-btn active" } else { "kind-btn" }
                                on:click=move |_| set_vis.set(Visibility::Public)
                            >"Public"</button>
                        </div>
                    </div>

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
