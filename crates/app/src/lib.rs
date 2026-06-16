pub mod app;
pub mod pages;
pub mod components;
pub mod auth;

#[cfg(feature = "ssr")]
pub mod server;
pub mod email;
#[cfg(feature = "ssr")]
pub mod mcp;
#[cfg(feature = "ssr")]
pub mod oauth_server;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use app::App;
    leptos::mount::hydrate_body(App);
}
