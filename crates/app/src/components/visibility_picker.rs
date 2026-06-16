use leptos::prelude::*;
use thanksgivings_core::VisibilityFilter;

#[component]
pub fn VisibilityPicker(
    value: ReadSignal<VisibilityFilter>,
    on_change: impl Fn(VisibilityFilter) + 'static + Clone,
) -> impl IntoView {
    let mine_cb = {
        let on_change = on_change.clone();
        move |_| on_change(VisibilityFilter::Mine)
    };
    let pub_cb = {
        let on_change = on_change.clone();
        move |_| on_change(VisibilityFilter::Public)
    };

    view! {
        <nav class="visibility-tabs">
            <button
                class=move || if value.get() == VisibilityFilter::Mine { "visibility-tab active" } else { "visibility-tab" }
                on:click=mine_cb
            >"Mine"</button>
            <button
                class=move || if value.get() == VisibilityFilter::Public { "visibility-tab active" } else { "visibility-tab" }
                on:click=pub_cb
            >"Public"</button>
        </nav>
    }
}
