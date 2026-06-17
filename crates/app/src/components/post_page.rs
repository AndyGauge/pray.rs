use leptos::prelude::*;
use thanksgivings_core::{Group, Post, PostKind, Visibility};
use time::format_description;

use crate::pages::book::share_prayer_to_group;

#[component]
pub fn PostPage(
    post: Post,
    /// The viewer's groups, offered as share targets.
    #[prop(optional, into)] groups: Signal<Vec<Group>>,
    /// Whether the viewer owns this prayer and may share it.
    #[prop(optional, into)] can_share: Signal<bool>,
) -> impl IntoView {
    let kind_label = match post.kind {
        PostKind::Prayer => "Prayer",
        PostKind::Praise => "Praise",
    };

    let date_fmt = format_description::parse("[month repr:long] [day], [year]")
        .unwrap_or_default();
    let date_str = post.created_at.format(&date_fmt).unwrap_or_default();

    let post_id        = post.id.to_string();
    let shared_with    = match post.visibility {
        Visibility::Group(gid) => Some(gid),
        _                      => None,
    };

    let (selected, set_selected) = signal(String::new());
    let (msg, set_msg)           = signal(Option::<(bool, String)>::None);

    let share = Action::new(move |(pid, gid): &(String, String)| {
        let pid = pid.clone();
        let gid = gid.clone();
        async move {
            match share_prayer_to_group(pid, gid).await {
                Ok(())  => (true, "Shared to group.".to_string()),
                Err(e)  => (false, e.to_string()),
            }
        }
    });

    Effect::new(move |_| {
        if let Some(result) = share.value().get() {
            set_msg.set(Some(result));
        }
    });

    let share_control = move || {
        if !can_share.get() {
            return None;
        }
        let gs = groups.get();
        if gs.is_empty() {
            return None;
        }
        let post_id = post_id.clone();
        Some(view! {
            <div class="share-row">
                <select
                    class="share-select"
                    on:change=move |e| set_selected.set(event_target_value(&e))
                >
                    <option value="" selected>"Share with a group…"</option>
                    {gs.into_iter().map(|g| {
                        let id = g.id.to_string();
                        let is_current = shared_with.map_or(false, |c| c == g.id);
                        let label = if is_current {
                            format!("{} ✓", g.name)
                        } else {
                            g.name
                        };
                        view! { <option value=id>{label}</option> }
                    }).collect_view()}
                </select>
                <button
                    class="share-btn"
                    disabled=move || share.pending().get()
                    on:click=move |_| {
                        let gid = selected.get();
                        if !gid.is_empty() {
                            set_msg.set(None);
                            share.dispatch((post_id.clone(), gid));
                        }
                    }
                >
                    {move || if share.pending().get() { "Sharing…" } else { "Share" }}
                </button>
            </div>
        })
    };

    view! {
        <article class="post-card">
            <p class="post-kind">{kind_label}</p>
            <blockquote class="post-content">{post.content}</blockquote>
            <p class="post-meta">{date_str}</p>
            {share_control}
            {move || msg.get().map(|(ok, m)| view! {
                <p class=if ok { "invite-ok" } else { "error-msg" }>{m}</p>
            })}
        </article>
    }
}
