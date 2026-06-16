use leptos::prelude::*;
use leptos_router::{components::A, hooks::{use_navigate, use_params_map}};
use thanksgivings_core::{ContactType, Group, GroupId, InviteSummary};

// ─── Server functions ─────────────────────────────────────────────────────────

#[server]
pub async fn fetch_my_groups() -> Result<Vec<Group>, ServerFnError> {
    use crate::server::AppState;
    use thanksgivings_core::UserId;
    use tower_sessions::Session;
    let state   = use_context::<AppState>().ok_or_else(|| ServerFnError::new("no state"))?;
    let session = leptos_axum::extract::<Session>().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    let uid: UserId = session.get("user_id").await.ok().flatten()
        .ok_or_else(|| ServerFnError::new("not authenticated"))?;
    thanksgivings_db::repository::groups::list_for_user(&state.db.pool, uid)
        .await.map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
pub async fn create_group(name: String) -> Result<Group, ServerFnError> {
    use crate::server::AppState;
    use thanksgivings_core::UserId;
    use tower_sessions::Session;
    let state   = use_context::<AppState>().ok_or_else(|| ServerFnError::new("no state"))?;
    let session = leptos_axum::extract::<Session>().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    let uid: UserId = session.get("user_id").await.ok().flatten()
        .ok_or_else(|| ServerFnError::new("not authenticated"))?;
    let name = name.trim().to_string();
    if name.is_empty() { return Err(ServerFnError::new("name required")); }
    thanksgivings_db::repository::groups::create(&state.db.pool, &name, uid)
        .await.map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
pub async fn send_invite(group_id: String, contact: String) -> Result<String, ServerFnError> {
    use crate::{email::mailer, server::AppState};
    use thanksgivings_core::{GroupId, UserId};
    use tower_sessions::Session;

    let state   = use_context::<AppState>().ok_or_else(|| ServerFnError::new("no state"))?;
    let session = leptos_axum::extract::<Session>().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    let uid: UserId = session.get("user_id").await.ok().flatten()
        .ok_or_else(|| ServerFnError::new("not authenticated"))?;

    let gid: GroupId = group_id.parse().map_err(|_| ServerFnError::new("invalid group"))?;
    let contact = contact.trim().to_string();
    if contact.is_empty() { return Err(ServerFnError::new("contact required")); }

    let is_phone = contact.chars().any(|c| c.is_ascii_digit()) && !contact.contains('@');
    let contact_type = if is_phone { ContactType::Phone } else { ContactType::Email };

    if contact_type == ContactType::Email {
        let parts: Vec<&str> = contact.splitn(2, '@').collect();
        let valid = parts.len() == 2 && !parts[0].is_empty() && parts[1].contains('.');
        if !valid { return Err(ServerFnError::new("Please enter a valid email address.")); }
    }

    let groups = thanksgivings_db::repository::groups::list_for_user(&state.db.pool, uid)
        .await.map_err(|e| ServerFnError::new(e.to_string()))?;
    let group = groups.iter().find(|g| g.id == gid)
        .ok_or_else(|| ServerFnError::new("group not found or not a member"))?;
    let inviter = thanksgivings_db::repository::users::fetch_by_id(&state.db.pool, uid)
        .await.map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("user not found"))?;

    let inv = thanksgivings_db::repository::invitations::create(
        &state.db.pool, gid, uid, &contact, &contact_type,
    ).await.map_err(|e| ServerFnError::new(e.to_string()))?;

    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "https://pray.rs".to_string());
    let invite_url = format!("{}/invite/{}", base_url, inv.token);

    match contact_type {
        ContactType::Email => {
            mailer::send_group_invite(&contact, &inviter.display_name, &group.name, &invite_url)
                .await.map_err(ServerFnError::new)?;
            Ok(invite_url)
        }
        ContactType::Phone => {
            let msg = format!(
                "{} invited you to join their prayer group on Thanksgivings: {}",
                inviter.display_name, invite_url
            );
            Ok(format!("sms:{contact}?body={}", urlencoding::encode(&msg)))
        }
    }
}

#[server]
pub async fn accept_invite(token: String) -> Result<String, ServerFnError> {
    use crate::server::AppState;
    use thanksgivings_core::UserId;
    use tower_sessions::Session;
    let state   = use_context::<AppState>().ok_or_else(|| ServerFnError::new("no state"))?;
    let session = leptos_axum::extract::<Session>().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    let uid: UserId = session.get("user_id").await.ok().flatten()
        .ok_or_else(|| ServerFnError::new("not authenticated"))?;
    thanksgivings_db::repository::invitations::accept(&state.db.pool, &token, uid)
        .await.map_err(|e| ServerFnError::new(e.to_string()))?
        .map(|gid| gid.to_string())
        .ok_or_else(|| ServerFnError::new("invitation not found or already used"))
}

#[server]
pub async fn get_group_join_token(group_id: String) -> Result<(String, String), ServerFnError> {
    use crate::server::AppState;
    use thanksgivings_core::{GroupId, UserId};
    use tower_sessions::Session;

    let state   = use_context::<AppState>().ok_or_else(|| ServerFnError::new("no state"))?;
    let session = leptos_axum::extract::<Session>().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    let uid: UserId = session.get("user_id").await.ok().flatten()
        .ok_or_else(|| ServerFnError::new("not authenticated"))?;

    let gid: GroupId = group_id.parse().map_err(|_| ServerFnError::new("invalid group"))?;

    let groups = thanksgivings_db::repository::groups::list_for_user(&state.db.pool, uid)
        .await.map_err(|e| ServerFnError::new(e.to_string()))?;
    if !groups.iter().any(|g| g.id == gid) {
        return Err(ServerFnError::new("not a member"));
    }

    let token = match thanksgivings_db::repository::group_join_tokens::get_for_group(&state.db.pool, gid)
        .await.map_err(|e| ServerFnError::new(e.to_string()))? {
        Some(t) => t,
        None    => thanksgivings_db::repository::group_join_tokens::create_or_replace(&state.db.pool, gid, uid)
            .await.map_err(|e| ServerFnError::new(e.to_string()))?,
    };

    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "https://pray.rs".to_string());
    let join_url = format!("{}/join/{}", base_url, token);
    Ok((token, join_url))
}

#[server]
pub async fn regenerate_group_join_token(group_id: String) -> Result<(String, String), ServerFnError> {
    use crate::server::AppState;
    use thanksgivings_core::{GroupId, UserId};
    use tower_sessions::Session;

    let state   = use_context::<AppState>().ok_or_else(|| ServerFnError::new("no state"))?;
    let session = leptos_axum::extract::<Session>().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    let uid: UserId = session.get("user_id").await.ok().flatten()
        .ok_or_else(|| ServerFnError::new("not authenticated"))?;

    let gid: GroupId = group_id.parse().map_err(|_| ServerFnError::new("invalid group"))?;

    let groups = thanksgivings_db::repository::groups::list_for_user(&state.db.pool, uid)
        .await.map_err(|e| ServerFnError::new(e.to_string()))?;
    if !groups.iter().any(|g| g.id == gid) {
        return Err(ServerFnError::new("not a member"));
    }

    let token = thanksgivings_db::repository::group_join_tokens::create_or_replace(&state.db.pool, gid, uid)
        .await.map_err(|e| ServerFnError::new(e.to_string()))?;

    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "https://pray.rs".to_string());
    let join_url = format!("{}/join/{}", base_url, token);
    Ok((token, join_url))
}

#[server]
pub async fn join_by_token(token: String) -> Result<(), ServerFnError> {
    use crate::server::AppState;
    use thanksgivings_core::UserId;
    use tower_sessions::Session;

    let state   = use_context::<AppState>().ok_or_else(|| ServerFnError::new("no state"))?;
    let session = leptos_axum::extract::<Session>().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    let uid: UserId = session.get("user_id").await.ok().flatten()
        .ok_or_else(|| ServerFnError::new("not authenticated"))?;

    let gid = thanksgivings_db::repository::group_join_tokens::find_group(&state.db.pool, &token)
        .await.map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("invalid or expired join link"))?;

    thanksgivings_db::repository::groups::add_member(&state.db.pool, gid, uid)
        .await.map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}

#[server]
pub async fn fetch_pending_invites(group_id: String) -> Result<Vec<InviteSummary>, ServerFnError> {
    use crate::server::AppState;
    use thanksgivings_core::{GroupId, UserId};
    use tower_sessions::Session;
    let state   = use_context::<AppState>().ok_or_else(|| ServerFnError::new("no state"))?;
    let session = leptos_axum::extract::<Session>().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    let _uid: UserId = session.get("user_id").await.ok().flatten()
        .ok_or_else(|| ServerFnError::new("not authenticated"))?;
    let gid: GroupId = group_id.parse().map_err(|_| ServerFnError::new("invalid group"))?;
    thanksgivings_db::repository::invitations::list_pending_for_group(&state.db.pool, gid)
        .await.map_err(|e| ServerFnError::new(e.to_string()))
}

// ─── Groups list page ─────────────────────────────────────────────────────────

#[component]
pub fn GroupsPage() -> impl IntoView {
    let navigate = use_navigate();
    let groups   = LocalResource::new(fetch_my_groups);
    let create   = ServerAction::<CreateGroup>::new();
    let (new_name, set_new_name) = signal(String::new());

    Effect::new(move |_| {
        if let Some(Ok(_)) = create.value().get() {
            groups.refetch();
            set_new_name.set(String::new());
        }
    });

    view! {
        <div class="book">
            <header class="book-header">
                <A href="/" attr:class="legal-back">"← Back"</A>
                <span class="book-title">"Prayer Groups"</span>
            </header>

            <div class="page-content">
                <Suspense fallback=|| view! { <p class="loading">"Loading..."</p> }>
                    {move || groups.get().map(|result| {
                        let gs = result.unwrap_or_default();
                        view! {
                            <div class="groups-list">
                                {if gs.is_empty() {
                                    view! {
                                        <p class="post-meta" style="text-align:center">
                                            "You have no groups yet. Create one below."
                                        </p>
                                    }.into_any()
                                } else {
                                    gs.into_iter().map(|g| {
                                        let id  = g.id.to_string();
                                        let nav = navigate.clone();
                                        view! {
                                            <button
                                                class="group-row"
                                                on:click=move |_| nav(
                                                    &format!("/groups/{id}"),
                                                    Default::default()
                                                )
                                            >
                                                <span class="group-name">{g.name}</span>
                                                <span class="group-arrow">"›"</span>
                                            </button>
                                        }
                                    }).collect_view().into_any()
                                }}
                            </div>
                        }
                    })}
                </Suspense>

                <div class="group-create">
                    <ActionForm action=create>
                        <div class="form-field">
                            <label>"New group name"</label>
                            <input
                                type="text"
                                name="name"
                                placeholder="e.g. Family, Bible study..."
                                prop:value=new_name
                                on:input=move |e| set_new_name.set(event_target_value(&e))
                                class="group-name-input"
                            />
                        </div>
                        <button type="submit" class="compose-btn">"Create group"</button>
                    </ActionForm>
                </div>
            </div>
        </div>
    }
}

// ─── QR section sub-components ───────────────────────────────────────────────

#[component]
fn GroupQrSection(group_id: String) -> impl IntoView {
    let gid_for_load = group_id.clone();

    let token_res: LocalResource<Result<(String, String), ServerFnError>> =
        LocalResource::new(move || get_group_join_token(gid_for_load.clone()));

    let (join_url, set_join_url)   = signal(String::new());
    let (qr_bust, set_qr_bust)     = signal(0u32);
    let (url_copied, set_url_copied) = signal(false);

    Effect::new(move |_| {
        if let Some(Ok((_, url))) = token_res.get() {
            set_join_url.set(url);
        }
    });

    view! {
        <div class="qr-section">
            <h3 class="qr-heading">"Share QR code"</h3>
            <p class="post-meta">"Anyone who scans this can join the group."</p>

            <Suspense fallback=|| view! { <p class="loading">"Generating link…"</p> }>
                {move || token_res.get().map(|res| match res {
                    Err(e) => {
                        let msg = e.to_string();
                        view! { <p class="error-msg">{msg}</p> }.into_any()
                    },
                    Ok(_) => view! {
                        <GroupQrLoaded
                            group_id=group_id.clone()
                            join_url=join_url
                            set_join_url=set_join_url
                            qr_bust=qr_bust
                            set_qr_bust=set_qr_bust
                            url_copied=url_copied
                            set_url_copied=set_url_copied
                        />
                    }.into_any(),
                })}
            </Suspense>
        </div>
    }
}

#[component]
fn GroupQrLoaded(
    group_id: String,
    join_url: ReadSignal<String>,
    set_join_url: WriteSignal<String>,
    qr_bust: ReadSignal<u32>,
    set_qr_bust: WriteSignal<u32>,
    url_copied: ReadSignal<bool>,
    set_url_copied: WriteSignal<bool>,
) -> impl IntoView {
    let gid_img   = group_id.clone();
    let gid_regen = group_id.clone();

    let qr_src = move || format!("/groups/{}/qr.svg?v={}", gid_img, qr_bust.get());

    let regen = Action::new(move |gid: &String| {
        let gid = gid.clone();
        async move {
            if let Ok((_, url)) = regenerate_group_join_token(gid).await {
                set_join_url.set(url);
                set_qr_bust.update(|n| *n += 1);
            }
        }
    });

    view! {
        <div class="qr-wrap">
            <img class="qr-img" src=qr_src alt="Join group QR code"/>
        </div>
        <div class="qr-url-row">
            <input
                type="text"
                class="api-key-input"
                readonly
                prop:value=join_url
            />
            <button
                class="api-key-copy-btn"
                on:click=move |_| {
                    #[cfg(target_arch = "wasm32")]
                    if let Some(w) = web_sys::window() {
                        let _ = w.navigator().clipboard().write_text(&join_url.get());
                        set_url_copied.set(true);
                        leptos::prelude::set_timeout(
                            move || set_url_copied.set(false),
                            std::time::Duration::from_millis(1500),
                        );
                    }
                }
            >
                {move || if url_copied.get() { "Copied!" } else { "Copy" }}
            </button>
        </div>
        <button
            class="qr-regen-btn"
            on:click=move |_| { let _ = regen.dispatch(gid_regen.clone()); }
        >
            "Regenerate link"
        </button>
    }
}

// ─── Group detail + invite page ───────────────────────────────────────────────

#[component]
pub fn GroupDetailPage() -> impl IntoView {
    let params   = use_params_map();
    let group_id = move || params.read().get("id").unwrap_or_default();

    let pending = LocalResource::new(move || fetch_pending_invites(group_id()));
    let (contact, set_contact)       = signal(String::new());
    let (result_msg, set_result_msg) = signal(Option::<(bool, String)>::None);
    let (sending, set_sending)       = signal(false);

    let send_action = Action::new(move |(gid, contact): &(String, String)| {
        let gid     = gid.clone();
        let contact = contact.clone();
        let pending = pending.clone();
        async move {
            match send_invite(gid, contact).await {
                Ok(url) if url.starts_with("sms:") => {
                    let _ = web_sys::window().and_then(|w| w.location().set_href(&url).ok());
                    (true, "Opening Messages…".to_string())
                }
                Ok(_) => { pending.refetch(); (true, "Invitation sent!".to_string()) }
                Err(e) => (false, e.to_string()),
            }
        }
    });

    Effect::new(move |_| {
        if let Some(result) = send_action.value().get() {
            set_result_msg.set(Some(result));
            set_sending.set(false);
            set_contact.set(String::new());
        }
    });

    view! {
        <div class="book">
            <header class="book-header">
                <A href="/groups" attr:class="legal-back">"← Groups"</A>
                <span class="book-title">"Invite"</span>
            </header>

            <div class="page-content">
                <div class="invite-form">
                    <h2 class="invite-heading">"Invite someone to pray with you"</h2>
                    <p class="post-meta">
                        "Enter an email address to send an invitation, or a phone number
                        to open a pre-filled text message."
                    </p>

                    <div class="form-field" style="margin-top:1.5rem">
                        <label>"Email or phone number"</label>
                        <input
                            type="text"
                            placeholder="friend@example.com or +1 555 0100"
                            prop:value=contact
                            on:input=move |e| set_contact.set(event_target_value(&e))
                            class="group-name-input"
                        />
                    </div>

                    {move || result_msg.get().map(|(ok, msg)| view! {
                        <p class=if ok { "invite-ok" } else { "error-msg" }>{msg}</p>
                    })}

                    <button
                        class="compose-btn"
                        disabled=sending
                        on:click=move |_| {
                            let c = contact.get();
                            if !c.is_empty() {
                                set_sending.set(true);
                                set_result_msg.set(None);
                                send_action.dispatch((group_id(), c));
                            }
                        }
                    >
                        {move || if sending.get() { "Sending…" } else { "Send invitation" }}
                    </button>
                </div>

                <Suspense fallback=|| ()>
                    {move || pending.get().map(|res| {
                        let invites = res.unwrap_or_default();
                        (!invites.is_empty()).then(|| view! {
                            <div class="pending-invites">
                                <h3 class="pending-heading">"Pending invitations"</h3>
                                {invites.into_iter().map(|inv| view! {
                                    <div class="pending-row">
                                        <span class="pending-contact">{inv.contact}</span>
                                        <span class="pending-badge">"awaiting"</span>
                                    </div>
                                }).collect_view()}
                            </div>
                        })
                    })}
                </Suspense>

                <GroupQrSection group_id=group_id()/>
            </div>
        </div>
    }
}

// ─── Join via QR page (/join/:token) ─────────────────────────────────────────

#[component]
pub fn JoinGroupPage() -> impl IntoView {
    let params   = use_params_map();
    let navigate = use_navigate();
    let token    = move || params.read().get("token").unwrap_or_default();
    let (error, set_error)     = signal(Option::<String>::None);
    let (joining, set_joining) = signal(false);

    let join_action = Action::new(move |tok: &String| {
        let tok = tok.clone();
        let nav = navigate.clone();
        async move {
            match join_by_token(tok).await {
                Ok(())  => { nav("/", Default::default()); None }
                Err(e)  => Some(e.to_string()),
            }
        }
    });

    Effect::new(move |_| {
        if let Some(Some(e)) = join_action.value().get() {
            set_error.set(Some(e));
            set_joining.set(false);
        }
    });

    view! {
        <div class="book">
            <div class="page-content login-page">
                <div class="login-wordmark"><crate::components::wordmark::Wordmark/></div>
                <h2 class="invite-heading">"Join a prayer group"</h2>
                <p class="post-meta">"You've been invited to pray together."</p>

                {move || error.get().map(|e| {
                    if e.contains("authenticated") {
                        view! {
                            <p class="post-meta" style="margin-top:1rem">
                                "Please " <a href="/login" style="color:var(--accent)">"sign in"</a>
                                " first to join the group."
                            </p>
                        }.into_any()
                    } else {
                        view! { <p class="error-msg">{e}</p> }.into_any()
                    }
                })}

                <div class="login-buttons">
                    <button
                        class="compose-btn"
                        disabled=joining
                        on:click=move |_| {
                            set_joining.set(true);
                            set_error.set(None);
                            let _ = join_action.dispatch(token());
                        }
                    >
                        {move || if joining.get() { "Joining…" } else { "Join group" }}
                    </button>
                    <A href="/" attr:class="compose-btn" attr:style="text-align:center;opacity:0.5">
                        "Maybe later"
                    </A>
                </div>
            </div>
        </div>
    }
}

// ─── Accept invite page (/invite/:token) ─────────────────────────────────────

#[component]
pub fn AcceptInvitePage() -> impl IntoView {
    let params   = use_params_map();
    let navigate = use_navigate();
    let token    = move || params.read().get("token").unwrap_or_default();
    let (error, set_error)       = signal(Option::<String>::None);
    let (accepting, set_accepting) = signal(false);

    let accept_action = Action::new(move |token: &String| {
        let token = token.clone();
        let nav   = navigate.clone();
        async move {
            match accept_invite(token).await {
                Ok(_)  => { nav("/", Default::default()); None }
                Err(e) => Some(e.to_string()),
            }
        }
    });

    Effect::new(move |_| {
        if let Some(Some(e)) = accept_action.value().get() {
            set_error.set(Some(e));
            set_accepting.set(false);
        }
    });

    view! {
        <div class="book">
            <div class="page-content login-page">
                <div class="login-wordmark"><crate::components::wordmark::Wordmark/></div>
                <p class="post-meta">"You've been invited to a prayer group."</p>

                {move || error.get().map(|e| view! { <p class="error-msg">{e}</p> })}

                <div class="login-buttons">
                    <button
                        class="compose-btn"
                        disabled=accepting
                        on:click=move |_| {
                            set_accepting.set(true);
                            accept_action.dispatch(token());
                        }
                    >
                        {move || if accepting.get() { "Joining…" } else { "Accept invitation" }}
                    </button>
                    <A href="/" attr:class="compose-btn" attr:style="text-align:center;opacity:0.5">
                        "Maybe later"
                    </A>
                </div>
            </div>
        </div>
    }
}
