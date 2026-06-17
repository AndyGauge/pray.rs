use leptos::prelude::*;
use leptos_router::components::A;

// ── Server functions ──────────────────────────────────────────────────────────

#[server]
pub async fn list_api_keys() -> Result<Vec<(String, String)>, ServerFnError> {
    use crate::server::AppState;
    use thanksgivings_core::UserId;
    use tower_sessions::Session;

    let state   = use_context::<AppState>().ok_or_else(|| ServerFnError::new("no state"))?;
    let session = leptos_axum::extract::<Session>().await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let uid: UserId = session.get("user_id").await.ok().flatten()
        .ok_or_else(|| ServerFnError::new("not authenticated"))?;

    let keys = thanksgivings_db::repository::api_keys::list(&state.db.pool, uid)
        .await.map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(keys.into_iter().map(|k| (k.id, k.name)).collect())
}

#[server]
pub async fn create_api_key(name: String) -> Result<String, ServerFnError> {
    use crate::server::AppState;
    use thanksgivings_core::UserId;
    use tower_sessions::Session;

    let state   = use_context::<AppState>().ok_or_else(|| ServerFnError::new("no state"))?;
    let session = leptos_axum::extract::<Session>().await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let uid: UserId = session.get("user_id").await.ok().flatten()
        .ok_or_else(|| ServerFnError::new("not authenticated"))?;

    let name = name.trim().to_string();
    if name.is_empty() { return Err(ServerFnError::new("name required")); }

    let (_id, raw) = thanksgivings_db::repository::api_keys::create(&state.db.pool, uid, &name)
        .await.map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(raw)
}

#[server]
pub async fn revoke_api_key(id: String) -> Result<(), ServerFnError> {
    use crate::server::AppState;
    use thanksgivings_core::UserId;
    use tower_sessions::Session;

    let state   = use_context::<AppState>().ok_or_else(|| ServerFnError::new("no state"))?;
    let session = leptos_axum::extract::<Session>().await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let uid: UserId = session.get("user_id").await.ok().flatten()
        .ok_or_else(|| ServerFnError::new("not authenticated"))?;

    thanksgivings_db::repository::api_keys::revoke(&state.db.pool, &id, uid)
        .await.map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}

// ── Setup instructions sub-component ─────────────────────────────────────────

#[component]
fn SetupInstructions(
    client: ReadSignal<usize>,
    set_client: WriteSignal<usize>,
    os: ReadSignal<usize>,
    set_os: WriteSignal<usize>,
) -> impl IntoView {
    view! {
        <div class="settings-section">
            <h2 class="settings-heading">"Connect your AI"</h2>
            <p class="post-meta" style="margin-bottom:1rem">
                "Currently supported: Claude. "
                "(ChatGPT and Gemini don't yet support MCP on their web interfaces.)"
            </p>

            <div class="setup-tabs">
                <button
                    class=move || if client.get() == 0 { "setup-tab setup-tab--active" } else { "setup-tab" }
                    on:click=move |_| set_client.set(0)
                >"Claude Code"</button>
                <button
                    class=move || if client.get() == 1 { "setup-tab setup-tab--active" } else { "setup-tab" }
                    on:click=move |_| set_client.set(1)
                >"Claude.ai (web)"</button>
                <button
                    class=move || if client.get() == 2 { "setup-tab setup-tab--active" } else { "setup-tab" }
                    on:click=move |_| set_client.set(2)
                >"Gemini CLI"</button>
            </div>

            {move || match client.get() {
                1 => view! { <ClaudeWebInstructions/> }.into_any(),
                2 => view! { <GeminiCliInstructions os=os set_os=set_os/> }.into_any(),
                _ => view! { <ClaudeCodeInstructions os=os set_os=set_os/> }.into_any(),
            }}
        </div>
    }
}

#[component]
fn ClaudeCodeInstructions(os: ReadSignal<usize>, set_os: WriteSignal<usize>) -> impl IntoView {
    view! {
        <div>
            // ── OAuth sign-in (recommended) ───────────────────────────
            <div class="setup-steps">
                <p class="post-meta">
                    "Recommended — sign in with your pray.rs account. No API key to manage."
                </p>
                <p class="post-meta" style="margin-top:0.75rem">"1. Add the MCP server:"</p>
                <code class="api-key-value" style="white-space:pre-wrap">
                    "claude mcp add --transport http prayers https://pray.rs/mcp"
                </code>
                <p class="post-meta" style="margin-top:0.75rem">
                    "2. In Claude Code, run " <code class="api-key-inline">"/mcp"</code>
                    ", select " <strong>"prayers"</strong> ", and choose "
                    <strong>"Authenticate"</strong>
                    ". A browser opens to sign in with Google or Facebook."
                </p>
                <p class="post-meta" style="margin-top:0.75rem">
                    "Added it without signing in? You'll only see public prayers. Run "
                    <code class="api-key-inline">"/mcp"</code> " → " <strong>"prayers"</strong>
                    " → " <strong>"Reauthenticate"</strong>
                    " to switch from public prayers to your own account."
                </p>
            </div>

            // ── API key (alternative) ─────────────────────────────────
            <p class="post-meta" style="margin-top:1.5rem;font-weight:600">
                "Or connect with a static API key"
            </p>
            <div class="setup-tabs" style="margin-top:0.5rem">
                <button
                    class=move || if os.get() == 0 { "setup-tab setup-tab--active" } else { "setup-tab" }
                    on:click=move |_| set_os.set(0)
                >"Mac / Linux"</button>
                <button
                    class=move || if os.get() == 1 { "setup-tab setup-tab--active" } else { "setup-tab" }
                    on:click=move |_| set_os.set(1)
                >"Windows"</button>
            </div>
            {move || if os.get() == 0 {
                view! {
                    <div class="setup-steps">
                        <p class="post-meta">
                            "1. Add to " <code class="api-key-inline">"~/.zshrc"</code>
                            " or " <code class="api-key-inline">"~/.bashrc"</code> ":"
                        </p>
                        <code class="api-key-value">"export PRAYERS_API_KEY=prs_..."</code>
                        <p class="post-meta" style="margin-top:0.75rem">"2. Register the MCP server:"</p>
                        <code class="api-key-value" style="white-space:pre-wrap">
                            {"claude mcp add prayers \\\n  --transport http https://pray.rs/mcp \\\n  --header \"Authorization: Bearer $PRAYERS_API_KEY\""}
                        </code>
                    </div>
                }.into_any()
            } else {
                view! {
                    <div class="setup-steps">
                        <p class="post-meta">
                            "1. Add to your PowerShell profile ("
                            <code class="api-key-inline">"$PROFILE"</code> "):"
                        </p>
                        <code class="api-key-value">{"$env:PRAYERS_API_KEY = \"prs_...\""}</code>
                        <p class="post-meta" style="margin-top:0.75rem">"2. Register the MCP server:"</p>
                        <code class="api-key-value" style="white-space:pre-wrap">
                            {"claude mcp add prayers `\n  --transport http https://pray.rs/mcp `\n  --header \"Authorization: Bearer $env:PRAYERS_API_KEY\""}
                        </code>
                    </div>
                }.into_any()
            }}
        </div>
    }
}

#[component]
fn GeminiCliInstructions(os: ReadSignal<usize>, set_os: WriteSignal<usize>) -> impl IntoView {
    view! {
        <div>
            <div class="setup-tabs" style="margin-top:0.5rem">
                <button
                    class=move || if os.get() == 0 { "setup-tab setup-tab--active" } else { "setup-tab" }
                    on:click=move |_| set_os.set(0)
                >"Mac / Linux"</button>
                <button
                    class=move || if os.get() == 1 { "setup-tab setup-tab--active" } else { "setup-tab" }
                    on:click=move |_| set_os.set(1)
                >"Windows"</button>
            </div>
            {move || if os.get() == 0 {
                view! {
                    <div class="setup-steps">
                        <p class="post-meta">
                            "1. Add to " <code class="api-key-inline">"~/.zshrc"</code>
                            " or " <code class="api-key-inline">"~/.bashrc"</code> ":"
                        </p>
                        <code class="api-key-value">"export PRAYERS_API_KEY=prs_..."</code>
                        <p class="post-meta" style="margin-top:0.75rem">"2. Register the MCP server:"</p>
                        <code class="api-key-value" style="white-space:pre-wrap">
                            {"gemini mcp add prayers \\\n  --transport http https://pray.rs/mcp \\\n  --header \"Authorization: Bearer $PRAYERS_API_KEY\""}
                        </code>
                    </div>
                }.into_any()
            } else {
                view! {
                    <div class="setup-steps">
                        <p class="post-meta">
                            "1. Add to your PowerShell profile ("
                            <code class="api-key-inline">"$PROFILE"</code> "):"
                        </p>
                        <code class="api-key-value">{"$env:PRAYERS_API_KEY = \"prs_...\""}</code>
                        <p class="post-meta" style="margin-top:0.75rem">"2. Register the MCP server:"</p>
                        <code class="api-key-value" style="white-space:pre-wrap">
                            {"gemini mcp add prayers `\n  --transport http https://pray.rs/mcp `\n  --header \"Authorization: Bearer $env:PRAYERS_API_KEY\""}
                        </code>
                    </div>
                }.into_any()
            }}
        </div>
    }
}

#[component]
fn ClaudeWebInstructions() -> impl IntoView {
    view! {
        <div class="setup-steps">
            <p class="post-meta">"Requires a Claude Pro, Team, or Enterprise account."</p>
            <ol class="setup-ol">
                <li>"Go to " <strong>"claude.ai → Settings → Integrations"</strong></li>
                <li>"Click " <strong>"Add MCP Server"</strong></li>
                <li>"Enter URL: " <code class="api-key-inline">"https://pray.rs/mcp"</code></li>
                <li>"Add header: " <code class="api-key-inline">"Authorization: Bearer prs_..."</code></li>
            </ol>
        </div>
    }
}

// ── Page ──────────────────────────────────────────────────────────────────────

#[component]
pub fn SettingsPage() -> impl IntoView {
    let keys     = LocalResource::new(|| list_api_keys());
    let (name, set_name)       = signal(String::new());
    let (new_key, set_new_key) = signal(Option::<String>::None);
    let (err, set_err)         = signal(Option::<String>::None);
    let (client, set_client)   = signal(0usize); // 0=Claude Code, 1=Claude.ai, 2=Gemini CLI
    let (os, set_os)           = signal(0usize); // 0=Mac/Linux, 1=Windows
    let (copied, set_copied)   = signal(false);

    let create = Action::new(move |n: &String| {
        let n = n.clone();
        let keys = keys.clone();
        async move {
            set_err.set(None);
            set_new_key.set(None);
            match create_api_key(n).await {
                Ok(raw) => { set_new_key.set(Some(raw)); set_name.set(String::new()); keys.refetch(); }
                Err(e)  => { set_err.set(Some(e.to_string())); }
            }
        }
    });

    let revoke = Action::new(move |id: &String| {
        let id   = id.clone();
        let keys = keys.clone();
        async move {
            set_new_key.set(None);
            let _ = revoke_api_key(id).await;
            keys.refetch();
        }
    });

    view! {
        <div class="book">
            <header class="book-header">
                <A href="/" attr:class="legal-back">"← Back"</A>
                <span class="book-title">"Connect your AI"</span>
            </header>

            <div class="page-content settings-page">

                // ── New key created ───────────────────────────────────────
                {move || new_key.get().map(|raw| {
                    let raw_copy = raw.clone();
                    view! {
                        <div class="api-key-new">
                            <p class="api-key-new-label">
                                "Your new API key — copy it now. You won't see it again."
                            </p>
                            <div class="api-key-copy-row">
                                <input
                                    type="text"
                                    class="api-key-input"
                                    readonly
                                    prop:value=raw
                                    on:click=move |e| {
                                        use wasm_bindgen::JsCast;
                                        let _ = e.target()
                                            .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                                            .map(|el| el.select());
                                    }
                                />
                                <button
                                    class="api-key-copy-btn"
                                    on:click=move |_| {
                                        #[cfg(target_arch = "wasm32")]
                                        if let Some(w) = web_sys::window() {
                                            let _ = w.navigator().clipboard().write_text(&raw_copy);
                                            set_copied.set(true);
                                            leptos::prelude::set_timeout(
                                                move || set_copied.set(false),
                                                std::time::Duration::from_millis(1500),
                                            );
                                        }
                                    }
                                >
                                    {move || if copied.get() { "Copied!" } else { "Copy" }}
                                </button>
                            </div>
                            <p class="api-key-instructions">
                                "MCP server URL: "
                                <code class="api-key-inline">"https://pray.rs/mcp"</code>
                            </p>
                        </div>
                    }
                })}

                // ── Error ─────────────────────────────────────────────────
                {move || err.get().map(|e| view! {
                    <p class="error-msg">{e}</p>
                })}

                // ── Create form ───────────────────────────────────────────
                <div class="settings-section">
                    <h2 class="settings-heading">"Generate a key"</h2>
                    <p class="post-meta">
                        "Give it a name so you know which client it belongs to."
                    </p>
                    <div class="form-field" style="margin-top:1rem">
                        <label>"Key name"</label>
                        <input
                            type="text"
                            placeholder="e.g. Claude Code, ChatGPT..."
                            prop:value=name
                            on:input=move |e| set_name.set(event_target_value(&e))
                            class="group-name-input"
                            on:keydown=move |e| {
                                if e.key() == "Enter" {
                                    let n = name.get();
                                    if !n.is_empty() { let _ = create.dispatch(n); }
                                }
                            }
                        />
                    </div>
                    <button
                        class="compose-btn"
                        style="margin-top:0.75rem"
                        on:click=move |_| {
                            let n = name.get();
                            if !n.is_empty() { let _ = create.dispatch(n); }
                        }
                    >
                        "Generate key"
                    </button>
                </div>

                // ── Existing keys ─────────────────────────────────────────
                <div class="settings-section">
                    <h2 class="settings-heading">"Active keys"</h2>
                    <Suspense fallback=|| view! { <p class="loading">"Loading..."</p> }>
                        {move || keys.get().map(|result| {
                            let ks = result.unwrap_or_default();
                            if ks.is_empty() {
                                view! {
                                    <p class="post-meta">"No keys yet."</p>
                                }.into_any()
                            } else {
                                ks.into_iter().map(|(id, key_name)| {
                                    let id_rev = id.clone();
                                    view! {
                                        <div class="api-key-row">
                                            <span class="api-key-name">{key_name}</span>
                                            <button
                                                class="api-key-revoke"
                                                on:click=move |_| { let _ = revoke.dispatch(id_rev.clone()); }
                                            >
                                                "Revoke"
                                            </button>
                                        </div>
                                    }
                                }).collect_view().into_any()
                            }
                        })}
                    </Suspense>
                </div>

                // ── How it works ──────────────────────────────────────────
                <SetupInstructions client=client set_client=set_client os=os set_os=set_os/>

                // ── Legal ─────────────────────────────────────────────────
                <div class="settings-section">
                    <h2 class="settings-heading">"Legal"</h2>
                    <nav class="settings-links">
                        <A href="/privacy"  attr:class="settings-link">"Privacy Policy"</A>
                        <A href="/terms"    attr:class="settings-link">"Terms of Service"</A>
                        <A href="/deletion" attr:class="settings-link">"Delete my data"</A>
                    </nav>
                </div>

            </div>
        </div>
    }
}
