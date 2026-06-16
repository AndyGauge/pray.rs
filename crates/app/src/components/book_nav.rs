use leptos::prelude::*;

#[component]
pub fn BookNav(
    page: ReadSignal<usize>,
    total: Signal<usize>,
    on_prev: impl Fn(web_sys::MouseEvent) + 'static,
    on_next: impl Fn(web_sys::MouseEvent) + 'static,
) -> impl IntoView {
    let at_start = move || page.get() == 0;
    let at_end   = move || page.get() + 1 >= total.get();

    view! {
        <div class="book-nav">
            <button class="nav-btn" disabled=at_start on:click=on_prev>"←"</button>
            <span class="page-counter">
                {move || {
                    let t = total.get();
                    if t == 0 {
                        "—".to_string()
                    } else {
                        format!("{} / {}", page.get() + 1, t)
                    }
                }}
            </span>
            <button class="nav-btn" disabled=at_end on:click=on_next>"→"</button>
        </div>
    }
}
