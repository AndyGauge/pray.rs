use leptos::prelude::*;
use leptos_meta::provide_meta_context;
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};

use crate::auth::LoginPage;
use crate::pages::{
    book::BookPage,
    compose::ComposePage,
    groups::{AcceptInvitePage, GroupDetailPage, GroupsPage, JoinGroupPage},
    legal::{DeletionPage, PrivacyPage, TermsPage},
    settings::SettingsPage,
};

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Router>
            <Routes fallback=|| "Page not found.">
                <Route path=path!("/")          view=BookPage/>
                <Route path=path!("/compose")   view=ComposePage/>
                <Route path=path!("/login")     view=LoginPage/>
                <Route path=path!("/auth/callback/:provider") view=crate::auth::CallbackPage/>
                <Route path=path!("/groups")          view=GroupsPage/>
                <Route path=path!("/groups/:id")      view=GroupDetailPage/>
                <Route path=path!("/invite/:token")   view=AcceptInvitePage/>
                <Route path=path!("/join/:token")     view=JoinGroupPage/>
                <Route path=path!("/settings") view=SettingsPage/>
                <Route path=path!("/privacy")  view=PrivacyPage/>
                <Route path=path!("/terms")    view=TermsPage/>
                <Route path=path!("/deletion") view=DeletionPage/>
            </Routes>
        </Router>
    }
}
