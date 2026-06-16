use leptos::prelude::*;

#[component]
pub fn CopyrightNotice() -> impl IntoView {
    #[cfg(target_arch = "wasm32")]
    let year = js_sys::Date::new_0().get_full_year() as i32;

    #[cfg(not(target_arch = "wasm32"))]
    let year = time::OffsetDateTime::now_utc().year();

    view! {
        <div class="book-copyright">{format!("© {year} Andrew Gauger")}</div>
    }
}
