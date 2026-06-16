use leptos::prelude::*;

#[component]
pub fn Wordmark() -> impl IntoView {
    view! {
        <span class="wordmark">
            "pray"
            <sup class="wordmark-e">"e"</sup>
            "rs"
        </span>
    }
}
