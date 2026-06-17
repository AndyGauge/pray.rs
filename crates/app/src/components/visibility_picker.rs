use leptos::prelude::*;
use thanksgivings_core::{Group, VisibilityFilter};

#[component]
pub fn VisibilityPicker(
    value: ReadSignal<VisibilityFilter>,
    /// The viewer's groups, each shown as its own tab.
    #[prop(optional, into)] groups: Signal<Vec<Group>>,
    on_change: impl Fn(VisibilityFilter) + 'static + Clone + Send + Sync,
) -> impl IntoView {
    let mine_cb = {
        let on_change = on_change.clone();
        move |_| on_change(VisibilityFilter::Mine)
    };
    let pub_cb = {
        let on_change = on_change.clone();
        move |_| on_change(VisibilityFilter::Public)
    };

    let group_tabs = {
        let on_change = on_change.clone();
        move || {
            groups.get().into_iter().map(|g| {
                let gid = g.id;
                let on_change = on_change.clone();
                view! {
                    <button
                        class=move || if value.get() == VisibilityFilter::Group(gid) {
                            "visibility-tab active"
                        } else {
                            "visibility-tab"
                        }
                        on:click=move |_| on_change(VisibilityFilter::Group(gid))
                    >{g.name}</button>
                }
            }).collect_view()
        }
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
            {group_tabs}
        </nav>
    }
}
