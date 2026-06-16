use leptos::prelude::*;
use thanksgivings_core::{Post, PostKind};
use time::format_description;

#[component]
pub fn PostPage(post: Post) -> impl IntoView {
    let kind_label = match post.kind {
        PostKind::Prayer => "Prayer",
        PostKind::Praise => "Praise",
    };

    let date_fmt = format_description::parse("[month repr:long] [day], [year]")
        .unwrap_or_default();
    let date_str = post.created_at.format(&date_fmt).unwrap_or_default();

    view! {
        <article class="post-card">
            <p class="post-kind">{kind_label}</p>
            <blockquote class="post-content">{post.content}</blockquote>
            <p class="post-meta">{date_str}</p>
        </article>
    }
}
