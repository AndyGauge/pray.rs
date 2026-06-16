use leptos::prelude::*;
use leptos_router::{components::A, hooks::use_query_map, NavigateOptions};

#[server(DeleteMyAccount, "/api")]
pub async fn delete_my_account() -> Result<(), ServerFnError> {
    use crate::server::AppState;
    use thanksgivings_core::UserId;
    use tower_sessions::Session;

    let state = use_context::<AppState>()
        .ok_or_else(|| ServerFnError::new("missing app state"))?;
    let session = leptos_axum::extract::<Session>().await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let user_id: Option<UserId> = session.get("user_id").await.ok().flatten();
    let uid = user_id.ok_or_else(|| ServerFnError::new("not authenticated"))?;

    thanksgivings_db::repository::users::delete_account(&state.db.pool, uid)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    session.flush().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[component]
pub fn PrivacyPage() -> impl IntoView {
    view! {
        <div class="legal-page">
            <div class="legal-content">
                <A href="/" attr:class="legal-back">"← Back"</A>
                <h1>"Privacy Policy"</h1>
                <p class="legal-date">"Last updated: June 2025"</p>

                <h2>"What this app is"</h2>
                <p>
                    "Thanksgivings (pray.rs) is a personal prayer book.
                    You write prayers and praises; the app keeps them for you."
                </p>

                <h2>"What we collect"</h2>
                <ul>
                    <li>
                        <strong>"Account information"</strong>
                        " — when you sign in with Google or Facebook we receive your name,
                        email address, and profile picture URL from that provider. We store
                        your email address so we can identify your account."
                    </li>
                    <li>
                        <strong>"Your prayers"</strong>
                        " — the text you write in the app, stored in our database."
                    </li>
                    <li>
                        <strong>"Session data"</strong>
                        " — a short-lived session cookie that keeps you logged in."
                    </li>
                </ul>
                <p>"We collect nothing else. No analytics, no tracking pixels, no ad networks."</p>

                <h2>"How we use it"</h2>
                <p>
                    "Your data is used solely to show you your own prayers. We do not sell,
                    share, or monetise your data in any way."
                </p>

                <h2>"Facebook Login"</h2>
                <p>
                    "If you sign in with Facebook, we receive only your name and email address
                    from Facebook. We do not access your posts, friends, timeline, or any other
                    Facebook data. We do not post to Facebook on your behalf."
                </p>
                <p>
                    "To revoke our access to your Facebook account, go to
                    Facebook → Settings → Apps and Websites and remove Thanksgivings."
                </p>

                <h2>"Data deletion"</h2>
                <p>
                    "To delete your account and all your prayers, email "
                    <a href="mailto:andygauge@gmail.com">"andygauge@gmail.com"</a>
                    " and we will remove your data within 7 days."
                </p>

                <h2>"Security"</h2>
                <p>
                    "Your data is stored on a private server. Connections to pray.rs are
                    encrypted with HTTPS. We do not store your OAuth tokens after login."
                </p>

                <h2>"Changes"</h2>
                <p>
                    "If we make material changes to this policy we will update the date above.
                    Continued use of the app after changes means you accept the new policy."
                </p>

                <h2>"Contact"</h2>
                <p>
                    "Questions? Email "
                    <a href="mailto:andygauge@gmail.com">"andygauge@gmail.com"</a>"."
                </p>
            </div>
        </div>
    }
}

#[component]
pub fn TermsPage() -> impl IntoView {
    view! {
        <div class="legal-page">
            <div class="legal-content">
                <A href="/" attr:class="legal-back">"← Back"</A>
                <h1>"Terms of Service"</h1>
                <p class="legal-date">"Last updated: June 2025"</p>

                <h2>"Acceptance"</h2>
                <p>
                    "By using Thanksgivings at pray.rs you agree to these terms. If you do not
                    agree, please do not use the app."
                </p>

                <h2>"What the app is"</h2>
                <p>
                    "Thanksgivings is a personal prayer journal. It is offered free of charge
                    as an alternative to social media — a quiet place for prayer and reflection,
                    not performance."
                </p>

                <h2>"Your content"</h2>
                <p>
                    "You own everything you write. We claim no rights to your prayers.
                    You are responsible for what you write. Do not use the app to store
                    content that is unlawful, harmful, or violates anyone else's rights."
                </p>

                <h2>"Acceptable use"</h2>
                <ul>
                    <li>"Use the app for personal prayer and reflection."</li>
                    <li>"Do not attempt to access other users' private prayers."</li>
                    <li>"Do not abuse or attempt to disrupt the service."</li>
                </ul>

                <h2>"Service availability"</h2>
                <p>
                    "This is a small personal project. We do our best to keep it running but
                    cannot guarantee uptime or data retention. Keep your own copies of anything
                    important."
                </p>

                <h2>"Termination"</h2>
                <p>
                    "We may suspend or close accounts that violate these terms. You may
                    delete your account at any time by emailing "
                    <a href="mailto:andygauge@gmail.com">"andygauge@gmail.com"</a>"."
                </p>

                <h2>"Limitation of liability"</h2>
                <p>
                    "The app is provided \"as is\" without warranty of any kind. We are not
                    liable for any loss of data or damages arising from use of the app."
                </p>

                <h2>"Changes"</h2>
                <p>
                    "We may update these terms. Continued use after changes means acceptance."
                </p>

                <h2>"Contact"</h2>
                <p>
                    "Questions? Email "
                    <a href="mailto:andygauge@gmail.com">"andygauge@gmail.com"</a>"."
                </p>
            </div>
        </div>
    }
}

#[component]
pub fn DeletionPage() -> impl IntoView {
    // Facebook passes ?id=<confirmation_code> when redirecting here after an
    // automated deletion signal — surface it as a confirmation reference.
    let query = use_query_map();
    let confirmation_id = move || query.read().get("id").unwrap_or_default();

    let navigate = leptos_router::hooks::use_navigate();
    let delete_action = ServerAction::<DeleteMyAccount>::new();
    let (confirmed, set_confirmed) = signal(false);
    let (pending_confirm, set_pending_confirm) = signal(false);

    // After the server action succeeds, show the done state and navigate away.
    Effect::new(move |_| {
        if let Some(Ok(())) = delete_action.value().get() {
            set_confirmed.set(true);
            set_pending_confirm.set(false);
        }
    });

    view! {
        <div class="legal-page">
            <div class="legal-content">
                <A href="/" attr:class="legal-back">"← Back"</A>
                <h1>"Delete My Data"</h1>

                // Facebook-sent confirmation code banner
                {move || {
                    let id = confirmation_id();
                    (!id.is_empty()).then(|| view! {
                        <div class="deletion-confirmed">
                            <p>
                                <strong>"Deletion request received."</strong>
                                " Your data will be removed within 7 days."
                            </p>
                            <p class="deletion-code">
                                "Confirmation code: " <code>{id}</code>
                            </p>
                        </div>
                    })
                }}

                // Post-delete success banner
                {move || confirmed.get().then(|| view! {
                    <div class="deletion-confirmed">
                        <p>
                            <strong>"Done."</strong>
                            " Your account and all your prayers have been deleted."
                        </p>
                    </div>
                })}

                <Show when=move || !confirmed.get()>
                    <h2>"Delete your account"</h2>
                    <p>
                        "This will permanently delete your account and every prayer you have written.
                        This cannot be undone."
                    </p>

                    // Two-step: first click shows confirm button, second click fires the action.
                    <Show
                        when=move || pending_confirm.get()
                        fallback=move || view! {
                            <button
                                class="deletion-btn"
                                on:click=move |_| set_pending_confirm.set(true)
                            >
                                "Delete my account and all my prayers"
                            </button>
                        }
                    >
                        <ActionForm action=delete_action>
                            <p class="deletion-warning">
                                "Are you sure? This is permanent."
                            </p>
                            <div class="deletion-actions">
                                <button type="submit" class="deletion-btn danger">
                                    "Yes, delete everything"
                                </button>
                                <button
                                    type="button"
                                    class="deletion-btn cancel"
                                    on:click=move |_| set_pending_confirm.set(false)
                                >
                                    "Cancel"
                                </button>
                            </div>
                        </ActionForm>
                    </Show>

                    <h2>"If you signed in with Facebook"</h2>
                    <p>"You can also revoke our access from Facebook directly:"</p>
                    <ol>
                        <li>"Go to Facebook → Settings → Security and Login."</li>
                        <li>"Scroll to \"Apps and Websites\" and click \"See more\"."</li>
                        <li>"Find Thanksgivings and click Remove."</li>
                    </ol>
                    <p>
                        "Revoking Facebook access signs you out but does not delete your prayers.
                        Use the button above if you want full account deletion."
                    </p>
                </Show>
            </div>
        </div>
    }
}
